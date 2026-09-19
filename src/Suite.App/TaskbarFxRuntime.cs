using Suite.Contracts;
using Suite.Platform;

namespace Suite.App;

/// <summary>
/// In-process taskbar appearance. Loads Native DLL; never starts TaskbarFx.exe.
/// </summary>
internal sealed class TaskbarFxRuntime : IDisposable
{
    private readonly TaskbarFxNativeBridge _native = new();
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
    private bool _disposed;

    public const int RestartMaxAttempts = 5;
    public static readonly TimeSpan RestartWindow = TimeSpan.FromSeconds(30);
    public static readonly TimeSpan WatchInterval = TimeSpan.FromSeconds(2);

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

    public TaskbarFxStatusDto Apply(TaskbarFxSettings settings)
    {
        lock (_gate)
        {
            _enabled = settings.Enabled;
            _mode = settings.ParsedMode;
            _argb = settings.Argb;
            if (!_enabled)
            {
                _mode = TaskbarAppearanceMode.Normal;
            }

            return ApplyLocked();
        }
    }

    public void StartWatch()
    {
        if (_disposed || _watchCts is not null)
        {
            return;
        }

        _watchCts = new CancellationTokenSource();
        _watchTask = Task.Run(() => WatchLoop(_watchCts.Token));
    }

    public void Dispose()
    {
        _disposed = true;
        _watchCts?.Cancel();
        try
        {
            _watchTask?.Wait(TimeSpan.FromSeconds(3));
        }
        catch (AggregateException)
        {
        }

        _watchCts?.Dispose();
        if (_watchTask is null || _watchTask.IsCompleted)
        {
            _native.Dispose();
        }
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

            lock (_gate)
            {
                if (!_enabled)
                {
                    _primaryHwnd = PrimaryHwnd();
                    continue;
                }

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
                        ExhaustRetries();
                    }

                    _primaryHwnd = IntPtr.Zero;
                    continue;
                }

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
                        ExhaustRetries();
                    }
                    else
                    {
                        ApplyLocked();
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

    private TaskbarFxStatusDto ApplyLocked()
    {
        if (!_native.IsLoaded && !_native.TryLoad())
        {
            _path = TaskbarFxIpc.PathUnavailable;
            _lastError = _native.LoadError;
            _message = _native.LoadError ?? TaskbarFxCopy.InitFailed;
            return Snapshot();
        }

        // After explorer recycle the XAML bridge can appear a second later than Shell_TrayWnd.
        TaskbarKind kind = _native.Probe();
        if (kind != TaskbarKind.ModernXaml && kind != TaskbarKind.NotFound)
        {
            System.Threading.Thread.Sleep(1500);
            TaskbarKind again = _native.Probe();
            if (again == TaskbarKind.ModernXaml || again == TaskbarKind.NotFound)
            {
                kind = again;
            }
        }

        switch (kind)
        {
            case TaskbarKind.ModernXaml:
                _path = TaskbarFxIpc.PathWin11Xaml;
                break;
            case TaskbarKind.NotFound:
                _path = TaskbarFxIpc.PathUnavailable;
                _lastError = TaskbarFxCopy.ApplyFailed("找不到任务栏。");
                _message = _lastError;
                return Snapshot();
            default:
                _path = TaskbarFxIpc.PathWin10Swca;
                break;
        }

        int code = _enabled && _mode != TaskbarAppearanceMode.Normal
            ? _native.Apply(_mode, _argb)
            : _native.Reset();

        if (code == 0)
        {
            _lastError = null;
            _message = !_enabled || _mode == TaskbarAppearanceMode.Normal
                ? TaskbarFxCopy.Closed
                : TaskbarFxCopy.Applied;
            _path = kind == TaskbarKind.ModernXaml
                ? TaskbarFxIpc.PathWin11Xaml
                : TaskbarFxIpc.PathWin10Swca;
        }
        else
        {
            _path = kind == TaskbarKind.ModernXaml
                ? TaskbarFxIpc.PathWin11Xaml
                : TaskbarFxIpc.PathUnavailable;
            _lastError = TaskbarFxCopy.ApplyFailed(ShortReason(code, _native.LastMessage()));
            _message = _lastError;
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

    private void ExhaustRetries()
    {
        _path = TaskbarFxIpc.PathUnavailable;
        _lastError = TaskbarFxCopy.ApplyFailed("任务栏窗口反复消失。");
        _message = _lastError;
        _restartWindowUtc = DateTime.MinValue;
    }

    private static IntPtr PrimaryHwnd()
    {
        IReadOnlyList<IntPtr> trays = TaskbarProbe.FindTrayWindows();
        return trays.Count > 0 ? trays[0] : IntPtr.Zero;
    }

    private static string ShortReason(int code, string nativeMessage) => code switch
    {
        1 => "系统不支持这种任务栏外观。",
        2 => "找不到任务栏。",
        4 => string.IsNullOrWhiteSpace(nativeMessage) ? "无法改任务栏外观。" : nativeMessage,
        5 => "档位无效。",
        6 => "任务栏效果被安全软件拦住，或 Win11 辅助模块没编出来。",
        _ => string.IsNullOrWhiteSpace(nativeMessage) ? "无法改任务栏外观。" : nativeMessage,
    };
}
