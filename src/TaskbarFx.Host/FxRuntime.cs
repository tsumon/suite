using Suite.Contracts;
using Suite.Platform;

namespace TaskbarFx.Host;

/// <summary>
/// Appearance policy for the helper process. Native DLL does the HWND work.
/// Explorer restart: re-apply with a retry budget. Never ends explorer.exe.
/// </summary>
internal sealed class FxRuntime : IDisposable
{
    private readonly NativeBridge _native = new();
    private readonly object _gate = new();
    private bool _enabled;
    private TaskbarAppearanceMode _mode = TaskbarAppearanceMode.Normal;
    private uint _argb = TaskbarFxSettings.DefaultArgb;
    private string _path = TaskbarFxIpc.PathUnavailable;
    private string? _lastError;
    private string? _message;
    private IntPtr _primaryHwnd;
    private int _restartAttempts;
    private DateTime _restartWindowUtc = DateTime.MinValue;
    private CancellationTokenSource? _watchCts;
    private Task? _watchTask;

    public const int RestartMaxAttempts = 5;
    public static readonly TimeSpan RestartWindow = TimeSpan.FromSeconds(30);
    public static readonly TimeSpan WatchInterval = TimeSpan.FromSeconds(2);

    public event EventHandler<TaskbarFxEnvelope>? EventRaised;

    public TaskbarFxStatusDto Status
    {
        get
        {
            lock (_gate)
            {
                return Snapshot();
            }
        }
    }

    public bool TryLoadNative(out string? error)
    {
        if (_native.TryLoad())
        {
            error = null;
            return true;
        }

        error = _native.LoadError;
        return false;
    }

    public TaskbarFxStatusDto Enable()
    {
        lock (_gate)
        {
            _enabled = true;
            return ApplyLocked();
        }
    }

    public TaskbarFxStatusDto Disable()
    {
        lock (_gate)
        {
            _enabled = false;
            _mode = TaskbarAppearanceMode.Normal;
            return ApplyLocked();
        }
    }

    public TaskbarFxStatusDto SetAppearance(TaskbarAppearanceMode mode, uint argb)
    {
        lock (_gate)
        {
            _mode = mode;
            _argb = argb;
            if (!_enabled && mode != TaskbarAppearanceMode.Normal)
            {
                _enabled = true;
            }

            if (mode == TaskbarAppearanceMode.Normal)
            {
                _enabled = false;
            }

            return ApplyLocked();
        }
    }

    public void StartWatch()
    {
        _watchCts = new CancellationTokenSource();
        _watchTask = Task.Run(() => WatchLoop(_watchCts.Token));
    }

    public void Dispose()
    {
        _watchCts?.Cancel();
        try
        {
            _watchTask?.Wait(TimeSpan.FromSeconds(1));
        }
        catch (AggregateException)
        {
        }

        _watchCts?.Dispose();
        _native.Dispose();
    }

    private async Task WatchLoop(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await Task.Delay(WatchInterval, cancellationToken).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                return;
            }

            TaskbarFxEnvelope? evt = null;
            lock (_gate)
            {
                if (!_enabled)
                {
                    _primaryHwnd = PrimaryHwnd();
                }
                else
                {
                    IntPtr now = PrimaryHwnd();
                    if (now == IntPtr.Zero)
                    {
                        if (_restartWindowUtc == DateTime.MinValue)
                        {
                            _restartWindowUtc = DateTime.UtcNow;
                            _restartAttempts = 0;
                        }
                        else if (DateTime.UtcNow - _restartWindowUtc > RestartWindow)
                        {
                            evt = ExhaustRetries();
                        }

                        _primaryHwnd = IntPtr.Zero;
                    }
                    else
                    {
                        bool hwndChanged = now != _primaryHwnd && _primaryHwnd != IntPtr.Zero;
                        bool reappeared = _primaryHwnd == IntPtr.Zero;
                        if (hwndChanged || reappeared)
                        {
                            if (_restartWindowUtc == DateTime.MinValue)
                            {
                                _restartWindowUtc = DateTime.UtcNow;
                                _restartAttempts = 0;
                            }

                            _restartAttempts++;
                            if (_restartAttempts > RestartMaxAttempts
                                || DateTime.UtcNow - _restartWindowUtc > RestartWindow)
                            {
                                evt = ExhaustRetries();
                            }
                            else
                            {
                                TaskbarFxStatusDto applied = ApplyLocked();
                                evt = TaskbarFxIpc.Event(TaskbarFxIpc.KindExplorerRestart, applied.Message, applied);
                            }
                        }
                        else
                        {
                            _restartWindowUtc = DateTime.MinValue;
                            _restartAttempts = 0;
                        }

                        _primaryHwnd = now;
                    }
                }
            }

            if (evt is not null)
            {
                EventRaised?.Invoke(this, evt);
            }
        }
    }

    private TaskbarFxStatusDto ApplyLocked()
    {
        if (!_native.IsLoaded && !_native.TryLoad())
        {
            _enabled = false;
            _path = TaskbarFxIpc.PathUnavailable;
            _lastError = _native.LoadError;
            _message = _native.LoadError;
            return Snapshot();
        }

        TaskbarKind kind = _native.Probe();
        switch (kind)
        {
            case TaskbarKind.ModernXaml:
                _path = TaskbarFxIpc.PathWin11Xaml;
                break;
            case TaskbarKind.NotFound:
                _enabled = false;
                _path = TaskbarFxIpc.PathUnavailable;
                _lastError = _native.LastMessage();
                _message = "当前系统找不到任务栏窗口。";
                return Snapshot();
            default:
                _path = TaskbarFxIpc.PathWin10Swca;
                break;
        }

        int code = _enabled || _mode != TaskbarAppearanceMode.Normal
            ? _native.Apply(_mode, _argb)
            : _native.Reset();

        if (code == 0)
        {
            _lastError = null;
            _message = _mode == TaskbarAppearanceMode.Normal
                ? "已恢复系统默认（没有结束 explorer）。"
                : (kind == TaskbarKind.ModernXaml ? "已应用到 Win11 任务栏。" : "已应用到经典任务栏。");
            _path = kind == TaskbarKind.ModernXaml
                ? TaskbarFxIpc.PathWin11Xaml
                : TaskbarFxIpc.PathWin10Swca;
        }
        else
        {
            _enabled = false;
            _lastError = MapError(code, _native.LastMessage());
            _message = _lastError;
            _path = kind == TaskbarKind.ModernXaml
                ? TaskbarFxIpc.PathWin11Xaml
                : TaskbarFxIpc.PathUnavailable;
        }

        _primaryHwnd = PrimaryHwnd();
        return Snapshot();
    }

    private TaskbarFxStatusDto Snapshot() => new()
    {
        Enabled = _enabled,
        Path = _path,
        Mode = TaskbarAppearanceModes.ToWire(_mode),
        Argb = _argb,
        Message = _message,
        LastError = _lastError,
    };

    private TaskbarFxEnvelope ExhaustRetries()
    {
        _enabled = false;
        _path = TaskbarFxIpc.PathUnavailable;
        _lastError = "Explorer restart retry budget exhausted (5 / 30s). Did not end explorer.";
        _message = _lastError;
        _restartWindowUtc = DateTime.MinValue;
        return TaskbarFxIpc.Event(TaskbarFxIpc.KindUnavailable, _lastError, Snapshot());
    }

    private static IntPtr PrimaryHwnd()
    {
        IReadOnlyList<IntPtr> trays = TaskbarProbe.FindTrayWindows();
        return trays.Count > 0 ? trays[0] : IntPtr.Zero;
    }

    private static string MapError(int code, string nativeMessage) => code switch
    {
        1 => string.IsNullOrWhiteSpace(nativeMessage)
            ? "SetWindowCompositionAttribute is not available."
            : nativeMessage,
        2 => "No taskbar window.",
        3 => string.IsNullOrWhiteSpace(nativeMessage)
            ? "Win11 XAML taskbar helper failed."
            : nativeMessage,
        4 => string.IsNullOrWhiteSpace(nativeMessage)
            ? "SetWindowCompositionAttribute failed."
            : nativeMessage,
        5 => "Unknown appearance mode.",
        6 => string.IsNullOrWhiteSpace(nativeMessage)
            ? "Win11 TAP helper missing or blocked."
            : nativeMessage,
        _ => nativeMessage,
    };
}
