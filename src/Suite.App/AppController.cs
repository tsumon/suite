using System.Threading;
using System.Runtime;
using Microsoft.Win32;
using Suite.Capture;
using Suite.Capture.Ocr;
using Suite.Contracts;
using Suite.NetSpeed;
using Suite.Pinboard;
using Suite.Platform;
using Suite.ThemeToggle;

namespace Suite.App;

public sealed class AppController : IDisposable
{
    private readonly SettingsStore _store;
    private readonly ThemeToggleService _theme;
    private readonly NetSpeedSampler _sampler;
    private readonly TrayIconService _tray;
    private readonly HotKeyHost _hotKey = new();
    private readonly NetSpeedWindow _overlay = new();
    private readonly NetSpeedTrayWidget _trayWidget = new();
    private readonly CaptureService _capture;
    private readonly IOcrService _ocr;
    private readonly string _ocrBackendLabel;
    private readonly IScrollCaptureService _scrollCapture = new ScrollCaptureService();
    private readonly PinboardService _pinboard = new();
    private CancellationTokenSource? _scrollCts;
    private ScrollCaptureSession? _scrollSession;
    private bool _scrollBusy;
    private readonly TaskbarFxClient _taskbarFx = new();
    private SettingsWindow? _settingsWindow;
    private HistoryWindow? _historyWindow;
    private PinManageWindow? _pinManageWindow;
    private ColorPickSession? _colorPick;
    private readonly List<NetSpeedTrayWidget> _secondaryOverlays = [];
    private CancellationTokenSource? _taskbarFxQueueCts;
    private string _lastStartLabel = "正常";
    private int _taskbarFxFailCount;
    private bool _disposed;
    private bool _applyingRunKey;
    private DateTime _lastCaptureStartUtc = DateTime.MinValue;

    public AppController()
    {
        _store = new SettingsStore();
        _store.Load();
        _theme = new ThemeToggleService(new RegistryPersonalizeStore(), new NativeThemeBroadcaster());
        _sampler = new NetSpeedSampler(new IpHelperInterfaceTable());
        _tray = new TrayIconService();
        _ocr = WindowsOcrService.TryCreate(out WindowsOcrService? winOcr) && winOcr is not null
            ? winOcr
            : new NullOcrService();
        _ocrBackendLabel = _ocr is WindowsOcrService w
            ? "WinOCR lang=" + (string.IsNullOrEmpty(w.LanguageTag) ? "?" : w.LanguageTag)
            : "NullOcr";
        _capture = new CaptureService(
            System.Windows.Application.Current.Dispatcher,
            _ocr,
            ReportStatus);
    }

    public AppSettings Settings => _store.Current;

    public IReadOnlyList<InterfaceSnapshot> ListAdapters()
    {
        try
        {
            return _sampler.ListAdapters();
        }
        catch (Exception ex)
        {
            _tray.Balloon("Suite", "无法读取网卡列表：" + ex.Message);
            return Array.Empty<InterfaceSnapshot>();
        }
    }

    public void Start()
    {
        AppSettings settings = _store.Current;
        CrashGuard.DegradeResult degrade = CrashGuard.OnStartup(settings);
        if (degrade.Degraded || degrade.SafeMode)
        {
            _store.Save(settings);
        }

        _lastStartLabel = degrade.LastStartLabel;
        SuiteLog.SetEnabled(settings.Advanced.LoggingEnabled);
        SuiteLog.Info("start label=" + _lastStartLabel);
        SuiteLog.Info("ocr backend=" + _ocrBackendLabel);

        _tray.ToggleThemeClicked += (_, _) => ToggleTheme();
        _tray.ToggleNetSpeedClicked += (_, _) => ToggleNetSpeedVisible();
        _tray.CaptureClicked += (_, _) => StartCapture();
        _tray.ScrollCaptureClicked += (_, _) => StartScrollCapture();
        _tray.ColorPickClicked += (_, _) => StartColorPick();
        _tray.HistoryClicked += (_, _) => ShowHistory();
        _tray.PinFromClipboardClicked += (_, _) => PinFromClipboard();
        _tray.PinManageClicked += (_, _) => ShowPinManage();
        _tray.ClearClickThroughClicked += (_, _) =>
        {
            _pinboard.ClearClickThrough();
            ReportStatus("已取消穿透。");
        };
        _tray.SettingsClicked += (_, _) => ShowSettings();
        _tray.StartWithWindowsChanged += (_, enabled) =>
        {
            if (_applyingRunKey)
            {
                return;
            }

            AppSettings next = Settings.Clone();
            next.StartWithWindows = enabled;
            ApplySettings(next);
        };
        _tray.ExitClicked += (_, _) => System.Windows.Application.Current.Shutdown();
        _tray.Sync(settings);

        // Previous Suite process may have left SetParent children under Shell_TrayWnd.
        int orphans = TaskbarEmbed.CleanupOrphanNetSpeedWindows(keepPid: Environment.ProcessId);
        if (orphans > 0)
        {
            SuiteLog.Info("destroyed orphan netspeed hwnds: " + orphans);
        }

        _pinboard.Changed += (_, _) => _tray.SetClickThroughAvailable(_pinboard.AnyClickThrough);
        _theme.ThemeChanged += (_, e) =>
        {
            _overlay.Dispatcher.Invoke(() =>
            {
                _overlay.ApplyTheme(e.Theme);
                _trayWidget.ApplyTheme(e.Theme);
                _settingsWindow?.ApplyTheme(e.Theme);
            });
            _pinboard.ApplyTheme(e.Theme);
            if (Settings.TaskbarFx.Enabled)
            {
                // Delay re-apply so ImmersiveColorSet / system chrome settle first.
                QueueTaskbarFx(Settings.TaskbarFx, shutdownWhenDisabled: false, delayMs: 500);
            }
        };
        if (_theme.TryRead(out ThemeKind theme, out _))
        {
            _overlay.ApplyTheme(theme);
            _trayWidget.ApplyTheme(theme);
            _pinboard.ApplyTheme(theme);
        }

        _overlay.Left = settings.NetSpeed.Left;
        _overlay.Top = settings.NetSpeed.Top;
        _overlay.ApplyAppearance(settings.NetSpeed);
        _trayWidget.ApplyAppearance(settings.NetSpeed);
        _overlay.PositionChangedByUser += (_, _) =>
        {
            _store.Update(s =>
            {
                s.NetSpeed.Left = _overlay.Left;
                s.NetSpeed.Top = _overlay.Top;
            });
        };
        _overlay.EmbedFailed += (_, message) =>
        {
            _tray.Balloon("Suite", message);
            _settingsWindow?.SetStatus(message);
        };
        _trayWidget.EmbedFailed += (_, message) =>
        {
            _tray.Balloon("Suite", message);
            _settingsWindow?.SetStatus(message);
        };
        void RequestFloatMode(object? _, EventArgs __)
        {
            AppSettings next = Settings.Clone();
            next.NetSpeed.EmbedInTaskbar = false;
            ApplySettings(next);
            _settingsWindow?.Reload();
        }
        _overlay.RequestFloat += RequestFloatMode;
        _trayWidget.RequestFloat += RequestFloatMode;

        _sampler.SetPreferredAdapter(settings.NetSpeed.IfIndex, settings.NetSpeed.AdapterAlias);
        _sampler.Sampled += (_, sample) =>
        {
            _ = _overlay.Dispatcher.InvokeAsync(() =>
            {
                _overlay.ApplySample(sample);
                _trayWidget.ApplySample(sample);
            });
            if (sample is { AdapterMissing: false, IfIndex: not 0 })
            {
                AppSettings current = Settings;
                if (current.NetSpeed.IfIndex != sample.IfIndex
                    || !string.Equals(current.NetSpeed.AdapterAlias, sample.Alias, StringComparison.Ordinal))
                {
                    _store.Update(s =>
                    {
                        s.NetSpeed.IfIndex = sample.IfIndex;
                        s.NetSpeed.AdapterAlias = sample.Alias;
                    });
                }
            }
        };
        _sampler.Start();
        SystemEvents.PowerModeChanged += OnPowerModeChanged;

        ApplyNetSpeedPresentation(settings.NetSpeed.Visible, settings.NetSpeed.EmbedInTaskbar);
        ApplySecondaryNetSpeed(settings);
        ApplyRunKey(settings.StartWithWindows);
        RegisterHotKey(settings.Hotkey);
        if (settings.TaskbarFx.Enabled)
        {
            // Give Win11 XAML taskbar bridge time to appear after explorer start.
            QueueTaskbarFx(settings.TaskbarFx, shutdownWhenDisabled: false, delayMs: 2000);
        }

        if (ProcessIntegrity.IsElevatedAdministrator())
        {
            _tray.Balloon("Suite", "当前以管理员运行。请用标准用户重新打开，以免开机启动异常。");
        }

        if (!string.IsNullOrEmpty(degrade.Message))
        {
            _tray.Balloon("Suite", degrade.Message!);
        }

        CrashGuard.MarkRunningFeatures(Settings);
    }

    public void ToggleTheme()
    {
        ThemeToggleResult result = _theme.Toggle();
        if (!result.Succeeded)
        {
            _tray.Balloon("Suite", "无法写入主题设置。可能被策略锁定。" + (result.Error is null ? "" : " " + result.Error));
            _settingsWindow?.SetStatus(result.Error ?? "主题写入失败");
            return;
        }

        string mode = result.NewTheme == ThemeKind.Light ? "浅色" : "深色";
        if (!result.BroadcastSucceeded)
        {
            _tray.Balloon("Suite", "已写入 " + mode + "，但广播未完成。未结束 explorer。");
            _settingsWindow?.SetStatus(result.Error ?? "广播未完成");
            return;
        }

        _tray.Balloon("Suite", "已切换为" + mode + "（Apps + System）");
        _settingsWindow?.SetStatus("主题已切换为" + mode);
        _settingsWindow?.RefreshLightTransparencyTip();
    }

    public void ToggleNetSpeedVisible()
    {
        AppSettings next = Settings.Clone();
        next.NetSpeed.Visible = !next.NetSpeed.Visible;
        ApplySettings(next);
    }

    public void ApplySettings(AppSettings settings)
    {
        AppSettings previous = Settings;
        _store.Save(settings);
        SuiteLog.SetEnabled(settings.Advanced.LoggingEnabled);
        _tray.Sync(settings);
        _sampler.SetPreferredAdapter(settings.NetSpeed.IfIndex, settings.NetSpeed.AdapterAlias);
        _overlay.ApplyAppearance(settings.NetSpeed);
        _trayWidget.ApplyAppearance(settings.NetSpeed);
        foreach (NetSpeedTrayWidget sec in _secondaryOverlays)
        {
            sec.ApplyAppearance(settings.NetSpeed);
        }

        ApplyNetSpeedPresentation(settings.NetSpeed.Visible, settings.NetSpeed.EmbedInTaskbar);
        ApplySecondaryNetSpeed(settings);
        CrashGuard.MarkRunningFeatures(settings);
        if (previous.StartWithWindows != settings.StartWithWindows)
        {
            ApplyRunKey(settings.StartWithWindows);
        }

        if (!HotkeyEquals(previous.Hotkey, settings.Hotkey)
            || !HotkeyEquals(previous.Capture.ColorPickHotkey, settings.Capture.ColorPickHotkey)
            || !HotkeyEquals(previous.Capture.PinClickThroughHotkey, settings.Capture.PinClickThroughHotkey)
            || !HotkeyEquals(previous.Capture.ScrollCaptureHotkey, settings.Capture.ScrollCaptureHotkey))
        {
            RegisterHotKey(settings.Hotkey);
        }

        if (TaskbarFxChanged(previous.TaskbarFx, settings.TaskbarFx))
        {
            QueueTaskbarFx(settings.TaskbarFx, shutdownWhenDisabled: true);
        }

        _settingsWindow?.RefreshLightTransparencyTip();
    }

    public void RestoreDefaults()
    {
        try
        {
            AppSettings defaults = AppSettings.CreateDefault();
            // Keep Run-key preference? Spec: factory defaults for hotkey/capture/netspeed appearance/taskbar.
            // CreateDefault covers all; ApplySettings writes + applies.
            ApplySettings(defaults);
            _settingsWindow?.Reload();
            _settingsWindow?.SetStatus("已恢复默认。");
        }
        catch
        {
            _settingsWindow?.SetStatus("没能恢复默认，请再试一次。");
        }
    }

    public void ReportUserStatus(string message) => ReportStatus(message);

    public string LastStartLabel => _lastStartLabel;

    private void ReportStatus(string message)
    {
        if (string.IsNullOrEmpty(message))
        {
            return;
        }

        try
        {
            _ = System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
            {
                _tray.Balloon("Suite", message);
                _settingsWindow?.SetStatus(message);
            });
        }
        catch
        {
        }
    }

    public void ShowSettings()
    {
        if (_settingsWindow is { IsVisible: true })
        {
            _settingsWindow.Activate();
            return;
        }

        _settingsWindow = new SettingsWindow(this);
        _settingsWindow.Closed += (_, _) => _settingsWindow = null;
        _settingsWindow.Show();
        if (!_hotKey.IsCaptureRegistered)
        {
            HotkeyBinding binding = Settings.Hotkey;
            IReadOnlyList<HotkeyBinding> alts = _hotKey.SuggestAlternates(binding, 3);
            string message = HotkeyCopy.CaptureFailed(
                binding.ToDisplayString(),
                _hotKey.LastCaptureWin32Error,
                alts.Select(a => a.ToDisplayString()).ToList());
            _settingsWindow.SetHotkeyConflict(message, alts);
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _taskbarFxQueueCts?.Cancel();
        _taskbarFxQueueCts?.Dispose();
        _taskbarFxQueueCts = null;
        SystemEvents.PowerModeChanged -= OnPowerModeChanged;
        _scrollCts?.Cancel();
        _scrollCts?.Dispose();
        _scrollSession?.Dispose();
        _scrollSession = null;
        _colorPick?.Dispose();
        _colorPick = null;
        _hotKey.Dispose();
        _capture.Dispose();
        _pinboard.CloseAll();
        _sampler.Dispose();
        _tray.Dispose();
        DisposeSecondaryOverlays();
        _trayWidget.HideEmbed();
        _trayWidget.Dispose();
        _overlay.DetachFromTaskbar();
        _overlay.Close();
        // Belt-and-suspenders: any titled chip still under tray after our dispose.
        _ = TaskbarEmbed.CleanupOrphanNetSpeedWindows(keepPid: null);
        _settingsWindow?.Close();
        _historyWindow?.Close();
        _pinManageWindow?.Close();
        CrashGuard.MarkCleanExit();
        try
        {
            AppSettings settings = Settings;
            if (settings.TaskbarFx.Enabled && settings.TaskbarFx.KeepAppearanceIfSuiteExits)
            {
                _taskbarFx.DisconnectKeepAliveAsync().GetAwaiter().GetResult();
            }
            else
            {
                TaskbarFxSettings off = settings.TaskbarFx.Clone();
                off.Enabled = false;
                _taskbarFx.ApplyAsync(off, shutdownWhenDisabled: true).GetAwaiter().GetResult();
            }
        }
        catch
        {
        }

        _taskbarFx.Dispose();
    }

    private void QueueTaskbarFx(TaskbarFxSettings settings, bool shutdownWhenDisabled, int delayMs = 0)
    {
        TaskbarFxSettings snapshot = settings.Clone();
        _taskbarFxQueueCts?.Cancel();
        _taskbarFxQueueCts?.Dispose();
        var queueCts = new CancellationTokenSource();
        _taskbarFxQueueCts = queueCts;
        CancellationToken token = queueCts.Token;
        _ = Task.Run(async () =>
        {
            try
            {
                if (delayMs > 0)
                {
                    await Task.Delay(delayMs, token).ConfigureAwait(false);
                }

                TaskbarFxStatusDto status;
                try
                {
                    status = await _taskbarFx.ApplyAsync(snapshot, shutdownWhenDisabled)
                        .ConfigureAwait(false);
                }
                catch (Exception ex)
                {
                    SuiteLog.Error("TaskbarFx.ApplyAsync", ex);
                    return; // FX must never take down Suite or netspeed.
                }
            string message = FormatTaskbarStatus(status);
            SuiteLog.Info(
                "taskbarFx path=" + status.Path
                + " enabled=" + status.Enabled
                + " err=" + (status.LastError ?? "")
                + " msg=" + (status.Message ?? ""));
            bool balloon = shutdownWhenDisabled && !string.IsNullOrEmpty(status.LastError);
            bool autoOff = false;
            if (!string.IsNullOrEmpty(status.LastError) && snapshot.Enabled)
            {
                int fails = Interlocked.Increment(ref _taskbarFxFailCount);
                if (fails >= 2 && Settings.Advanced.TaskbarFxAutoDisableOnFail)
                {
                    autoOff = true;
                }
            }
            else if (string.IsNullOrEmpty(status.LastError) && snapshot.Enabled)
            {
                Interlocked.Exchange(ref _taskbarFxFailCount, 0);
            }

                try
                {
                    await System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
                    {
                        if (token.IsCancellationRequested)
                        {
                            return;
                        }
                    _settingsWindow?.SetTaskbarStatus(message);
                    if (balloon)
                    {
                        _tray.Balloon("Suite", message);
                    }

                    if (autoOff)
                    {
                        // FX only — never hide or float netspeed because of FX failure.
                        AppSettings next = Settings.Clone();
                        next.TaskbarFx.Enabled = false;
                        _store.Save(next);
                        _tray.Sync(next);
                        _settingsWindow?.Reload();
                        string off = CrashGuard.MsgFxOff;
                        _tray.Balloon("Suite", off);
                        _settingsWindow?.SetStatus(off);
                        Interlocked.Exchange(ref _taskbarFxFailCount, 0);
                    }

                    // Re-assert netspeed after FX inject/retry (TAP can orphan SetParent children).
                    AppSettings live = Settings;
                    if (live.NetSpeed.Visible)
                    {
                        ApplyNetSpeedPresentation(live.NetSpeed.Visible, live.NetSpeed.EmbedInTaskbar);
                        ApplySecondaryNetSpeed(live);
                    }
                    });
                }
                catch
                {
                }
            }
            catch (OperationCanceledException)
            {
            }
        });
    }

    public static string FormatTaskbarStatus(TaskbarFxStatusDto status) => TaskbarFxCopy.FormatStatus(status);

    private static bool TaskbarFxChanged(TaskbarFxSettings a, TaskbarFxSettings b) =>
        a.Enabled != b.Enabled
        || a.Argb != b.Argb
        || !string.Equals(a.Mode, b.Mode, StringComparison.OrdinalIgnoreCase)
        || a.KeepAppearanceIfSuiteExits != b.KeepAppearanceIfSuiteExits;

    private void OnPowerModeChanged(object sender, PowerModeChangedEventArgs e)
    {
        if (e.Mode == PowerModes.Resume)
        {
            _sampler.ResetAfterResume();
        }
    }

    private void ApplyNetSpeedPresentation(bool visible, bool embedInTaskbar)
    {
        if (!visible)
        {
            _trayWidget.HideEmbed();
            _overlay.DetachFromTaskbar();
            _overlay.Hide();
            return;
        }

        if (!embedInTaskbar)
        {
            // Float mode: WPF window. Hide GDI embed chip.
            _trayWidget.HideEmbed();
            _overlay.ShowAsFloating();
            return;
        }

        // Embed mode: WinForms ULW chip. Hide WPF overlay hard (Opacity=0 + Hide) so it cannot ghost.
        _overlay.DetachFromTaskbar();
        try
        {
            _overlay.Opacity = 0;
            _overlay.ShowInTaskbar = false;
            if (_overlay.IsVisible)
            {
                _overlay.Hide();
            }
        }
        catch
        {
        }

        _ = TaskbarEmbed.CleanupOrphanNetSpeedWindows(keepPid: Environment.ProcessId);

        if (_trayWidget.TryEmbed(out string? error) && _trayWidget.IsVisuallyPresent())
        {
            return;
        }

        if (_trayWidget.TryEmbed(out error) && _trayWidget.IsVisuallyPresent())
        {
            return;
        }

        string message = "网速嵌入失败，仍保留任务栏槽并重试。" + (string.IsNullOrWhiteSpace(error) ? "" : " " + error);
        _tray.Balloon("Suite", message);
        _settingsWindow?.SetStatus(message);
        SuiteLog.Info("netspeed gdi embed failed: " + (error ?? ""));
        // Heartbeat on the WinForms widget keeps retrying SetParent — never leave a missing chip.
    }

    private void ApplyRunKey(bool enabled)
    {
        string exe = Environment.ProcessPath ?? "";
        _applyingRunKey = true;
        try
        {
            if (!RunKeyService.TrySetEnabled(enabled, exe, out string? error))
            {
                _tray.Balloon("Suite", "开机启动写入失败：" + error);
                _settingsWindow?.SetStatus(error ?? "Run 键写入失败");
            }
        }
        finally
        {
            _applyingRunKey = false;
            _tray.Sync(Settings);
        }
    }

    private void RegisterHotKey(HotkeyBinding binding)
    {
        AppSettings settings = Settings;
        _hotKey.Pressed -= OnHotKeyPressed;
        _hotKey.Pressed += OnHotKeyPressed;
        _hotKey.PinClipboardPressed -= OnPinClipboardHotKey;
        _hotKey.PinClipboardPressed += OnPinClipboardHotKey;
        _hotKey.ColorPickPressed -= OnColorPickHotKey;
        _hotKey.ColorPickPressed += OnColorPickHotKey;
        _hotKey.PinClickThroughPressed -= OnPinClickThroughHotKey;
        _hotKey.PinClickThroughPressed += OnPinClickThroughHotKey;
        _hotKey.ScrollCapturePressed -= OnScrollCaptureHotKey;
        _hotKey.ScrollCapturePressed += OnScrollCaptureHotKey;
        if (!_hotKey.TryStart(
                binding,
                settings.Capture.ColorPickHotkey,
                settings.Capture.PinClickThroughHotkey,
                settings.Capture.ScrollCaptureHotkey,
                out _,
                out string? pinError,
                out string? colorPickError,
                out string? clickThroughError,
                out string? scrollCaptureError))
        {
            IReadOnlyList<HotkeyBinding> alts = _hotKey.SuggestAlternates(binding, 3);
            string message = HotkeyCopy.CaptureFailed(
                binding.ToDisplayString(),
                _hotKey.LastCaptureWin32Error,
                alts.Select(a => a.ToDisplayString()).ToList());
            _tray.Balloon("Suite", message);
            _settingsWindow?.SetHotkeyConflict(message, alts);
        }
        else
        {
            _settingsWindow?.ClearHotkeyConflict();
        }

        if (!string.IsNullOrEmpty(pinError))
        {
            IReadOnlyList<HotkeyBinding> alts = _hotKey.SuggestAlternates(
                new HotkeyBinding { VirtualKey = HotkeyBinding.DefaultVirtualKey },
                3);
            string pinMessage = HotkeyCopy.PinFailed(
                _hotKey.LastPinWin32Error,
                alts.Select(a => a.ToDisplayString()).ToList());
            _tray.Balloon("Suite", pinMessage);
            _settingsWindow?.SetStatus(pinMessage);
        }

        if (!string.IsNullOrEmpty(colorPickError))
        {
            string msg = "取色热键注册失败，请换一组键。";
            _tray.Balloon("Suite", msg);
            _settingsWindow?.SetStatus(msg);
        }

        if (!string.IsNullOrEmpty(clickThroughError))
        {
            string msg = "穿透快捷键注册失败，请换一组键。";
            _tray.Balloon("Suite", msg);
            _settingsWindow?.SetStatus(msg);
        }

        if (!string.IsNullOrEmpty(scrollCaptureError))
        {
            string msg = "滚动长截图热键注册失败，请换一组键。";
            _tray.Balloon("Suite", msg);
            _settingsWindow?.SetStatus(msg);
        }
    }

    /// <summary>Settings 「试用下一可用键」: probe once, apply, no steal loop.</summary>
    public bool TryApplyNextAvailableHotkey(out string status)
    {
        AppSettings current = Settings;
        HotkeyBinding? next = _hotKey.FindNextAvailableAfter(current.Hotkey);
        if (next is null)
        {
            status = HotkeyCopy.NoFreeKey;
            return false;
        }

        AppSettings updated = current.Clone();
        updated.Hotkey = next;
        ApplySettings(updated);
        _settingsWindow?.Reload();
        if (!_hotKey.IsCaptureRegistered)
        {
            // RegisterHotKey already wrote conflict copy to tray/status.
            status = HotkeyCopy.CaptureFailed(
                next.ToDisplayString(),
                _hotKey.LastCaptureWin32Error,
                _hotKey.SuggestAlternates(next, 3).Select(a => a.ToDisplayString()).ToList());
            return false;
        }

        status = HotkeyCopy.AppliedAlternate(next.ToDisplayString());
        _settingsWindow?.SetStatus(status);
        _settingsWindow?.ClearHotkeyConflict();
        return true;
    }

    public void StartScrollCapture()
    {
        if (_capture.IsBusy || _scrollBusy)
        {
            return;
        }

        _scrollBusy = true;
        _scrollSession?.Dispose();
        _scrollCts?.Cancel();
        _scrollCts?.Dispose();
        _scrollCts = new CancellationTokenSource();
        AppSettings settingsSnapshot = Settings;
        var session = new ScrollCaptureSession(
            ReportStatus,
            _scrollCapture,
            new ScrollCaptureOptions(),
            result => OnScrollCaptureFinished(result, settingsSnapshot),
            System.Windows.Application.Current.Dispatcher);
        _scrollSession = session;
        // Bridge AppController cancel → session (Esc also handled inside session).
        _scrollCts.Token.Register(() =>
        {
            try
            {
                session.Cancel();
            }
            catch
            {
            }
        });
        session.Begin();
    }

    private void OnScrollCaptureFinished(ScrollCaptureResult result, AppSettings settingsSnapshot)
    {
        void Handle()
        {
            try
            {
                if (!result.Succeeded || result.Image is null)
                {
                    if (!string.IsNullOrEmpty(result.Error))
                    {
                        ReportStatus(result.Error!);
                    }

                    result.Image?.ReleasePixels();
                    GCSettings.LargeObjectHeapCompactionMode = GCLargeObjectHeapCompactionMode.CompactOnce;
                    GC.Collect(2, GCCollectionMode.Aggressive, blocking: true, compacting: true);
                    return;
                }

                string? saved = null;
                string? error = null;
                try
                {
                    ImageClipboard.Copy(result.Image);
                }
                catch
                {
                    error = ScrollCaptureService.CaptureFailed;
                }

                if (error is null && settingsSnapshot.Capture.SaveFileAfterCapture)
                {
                    try
                    {
                        saved = PngFileSaver.Save(result.Image, settingsSnapshot.Capture.SaveDirectory);
                    }
                    catch (Exception ex)
                    {
                        error = "无法保存文件：" + ex.Message;
                    }
                }

                result.Image.ReleasePixels();
                GCSettings.LargeObjectHeapCompactionMode = GCLargeObjectHeapCompactionMode.CompactOnce;
                GC.Collect(2, GCCollectionMode.Aggressive, blocking: true, compacting: true);
                if (error is not null)
                {
                    ReportStatus(error);
                    return;
                }

                ReportStatus(saved is null
                    ? ScrollCaptureService.Copied
                    : ScrollCaptureService.CopiedAndSaved);
            }
            finally
            {
                _scrollBusy = false;
                _scrollSession = null;
            }
        }

        if (System.Windows.Application.Current.Dispatcher.CheckAccess())
        {
            Handle();
        }
        else
        {
            System.Windows.Application.Current.Dispatcher.Invoke(Handle);
        }
    }

    public void StartCapture()
    {
        if (_capture.IsBusy || _scrollBusy)
        {
            return;
        }

        // Swallow F1 auto-repeat / duplicate hotkey bursts (seen as multi-flash on open).
        DateTime now = DateTime.UtcNow;
        if ((now - _lastCaptureStartUtc).TotalMilliseconds < 450)
        {
            return;
        }

        _lastCaptureStartUtc = now;
        AppSettings settings = Settings;
        _capture.Begin(
            new CaptureRequest
            {
                SaveFile = settings.Capture.SaveFileAfterCapture,
                SaveDirectory = settings.Capture.SaveDirectory,
                PinAfterCapture = settings.Capture.PinAfterCapture,
                ShowMagnifier = settings.Capture.ShowMagnifier,
            },
            OnCaptureFinished);
    }

    public void PinFromClipboard()
    {
        if (_capture.IsBusy || _scrollBusy)
        {
            return;
        }

        if (!_pinboard.TryPinFromClipboard(out string? error))
        {
            _tray.Balloon("Suite", error ?? "剪贴板里没有图片。");
        }
    }

    private void OnHotKeyPressed(object? sender, EventArgs e) => StartCapture();

    private void OnPinClipboardHotKey(object? sender, EventArgs e) => PinFromClipboard();

    private void OnCaptureFinished(CaptureResult result)
    {
        if (result.Cancelled)
        {
            return;
        }

        if (!string.IsNullOrEmpty(result.Error))
        {
            _tray.Balloon("Suite", result.Error);
            _settingsWindow?.SetStatus(result.Error);
        }

        if (result.Image is not null)
        {
            _ = ScreenshotHistory.TryAdd(result.Image, Settings);
        }

        if (result.Image is not null && result.PinRequested)
        {
            uint dpiX = result.DpiX == 0 ? 96u : result.DpiX;
            uint dpiY = result.DpiY == 0 ? 96u : result.DpiY;
            double? left = null;
            double? top = null;
            if (!result.Selection.IsEmpty)
            {
                (left, top) = DipConvert.Origin(result.Selection.X, result.Selection.Y, dpiX, dpiY);
            }

            _pinboard.Pin(result.Image, left, top, dpiX, dpiY);
        }
        // Non-pin captures: history wrote PNG to disk; do not keep the BitmapSource rooted via locals past this frame.
        // (Pin path intentionally retains the same instance inside PinWindow until closed.)
    }


    private void OnColorPickHotKey(object? sender, EventArgs e) => StartColorPick();

    private void OnScrollCaptureHotKey(object? sender, EventArgs e) => StartScrollCapture();

    private void OnPinClickThroughHotKey(object? sender, EventArgs e)
    {
        if (_pinboard.TryToggleTopClickThrough(out string? error))
        {
            return;
        }

        ReportStatus(error ?? "当前没有贴图。");
    }

    public void StartColorPick()
    {
        if (_capture.IsBusy || _scrollBusy)
        {
            return;
        }

        _colorPick?.Dispose();
        _colorPick = new ColorPickSession(ReportStatus);
        _colorPick.Begin();
    }

    public void ShowHistory()
    {
        if (_historyWindow is { IsVisible: true })
        {
            _historyWindow.Activate();
            return;
        }

        _historyWindow = new HistoryWindow(this);
        _historyWindow.Closed += (_, _) => _historyWindow = null;
        _historyWindow.Show();
    }

    public void ShowPinManage()
    {
        if (_pinManageWindow is { IsVisible: true })
        {
            _pinManageWindow.Activate();
            return;
        }

        _pinManageWindow = new PinManageWindow(this);
        _pinManageWindow.Closed += (_, _) => _pinManageWindow = null;
        _pinManageWindow.Show();
    }

    public IReadOnlyList<PinInfo> ListPins() => _pinboard.List();

    public void FocusPin(int oneBasedIndex)
    {
        PinInfo? info = _pinboard.List().FirstOrDefault(p => p.Index == oneBasedIndex);
        if (info is not null)
        {
            _pinboard.Focus(info.Window);
        }
    }

    public void TogglePinThrough(int oneBasedIndex)
    {
        PinInfo? info = _pinboard.List().FirstOrDefault(p => p.Index == oneBasedIndex);
        info?.Window.SetClickThrough(!info.Window.IsClickThrough);
    }

    public void ClosePin(int oneBasedIndex)
    {
        PinInfo? info = _pinboard.List().FirstOrDefault(p => p.Index == oneBasedIndex);
        if (info is not null)
        {
            _pinboard.Close(info.Window);
        }
    }

    public void CloseAllPins() => _pinboard.CloseAll();

    public bool TryPinHistory(string path, out string? error)
    {
        error = null;
        if (!ScreenshotHistory.TryLoadBitmap(path, out System.Windows.Media.Imaging.BitmapSource? image, out error) || image is null)
        {
            return false;
        }

        try
        {
            uint dpi = PinNative.DpiNearCursor();
            _pinboard.Pin(image, null, null, dpi, dpi);
            return true;
        }
        catch
        {
            error = ScreenshotHistory.PinFail;
            return false;
        }
    }

    public async Task CheckUpdatesAsync()
    {
        ReportStatus(UpdateChecker.Checking);
        UpdateChecker.CheckResult result = await UpdateChecker.CheckAsync(Settings.Update).ConfigureAwait(true);
        await System.Windows.Application.Current.Dispatcher.InvokeAsync(() =>
        {
            ReportStatus(result.Message);
            if (!result.HasUpdate || string.IsNullOrEmpty(result.HtmlUrl))
            {
                return;
            }

            System.Windows.MessageBoxResult answer = System.Windows.MessageBox.Show(
                result.Message + Environment.NewLine + "通道：" + (result.ChannelZh ?? "") + Environment.NewLine + "版本：" + (result.Version ?? ""),
                "Suite 更新",
                System.Windows.MessageBoxButton.OKCancel,
                System.Windows.MessageBoxImage.Information);
            if (answer != System.Windows.MessageBoxResult.OK)
            {
                return;
            }

            try
            {
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
                {
                    FileName = result.HtmlUrl,
                    UseShellExecute = true,
                });
            }
            catch
            {
                ReportStatus(UpdateChecker.FailDownload);
            }
        });
    }

    public bool TrySwitchToAcrylicReadable(out string status)
    {
        try
        {
            AppSettings next = Settings.Clone();
            next.TaskbarFx.Enabled = true;
            if (!string.Equals(next.TaskbarFx.Mode, TaskbarAppearanceModes.Acrylic, StringComparison.OrdinalIgnoreCase))
            {
                next.TaskbarFx.Mode = TaskbarAppearanceModes.Acrylic;
            }

            next.NetSpeed.BackgroundTransparent = false;
            if (next.NetSpeed.DownColorArgb is null)
            {
                next.NetSpeed.DownColorArgb = 0xFF1A1A1A;
            }

            if (next.NetSpeed.UpColorArgb is null)
            {
                next.NetSpeed.UpColorArgb = 0xFF1A1A1A;
            }

            ApplySettings(next);
            status = "已改用亚克力方案。";
            return true;
        }
        catch
        {
            status = "没法应用亚克力方案，请手动改任务栏效果。";
            return false;
        }
    }

    private void ApplySecondaryNetSpeed(AppSettings settings)
    {
        DisposeSecondaryOverlays();
        if (!settings.NetSpeed.Visible || !settings.NetSpeed.EmbedInTaskbar || !settings.NetSpeed.EmbedSecondary)
        {
            return;
        }

        IReadOnlyList<IntPtr> trays = TaskbarProbe.FindTrayWindows();
        IntPtr primary = TaskbarEmbed.FindPrimaryTray();
        int attempted = 0;
        int failed = 0;
        ThemeKind theme = ThemeKind.Light;
        _ = _theme.TryRead(out theme, out _);
        foreach (IntPtr tray in trays)
        {
            if (tray == IntPtr.Zero || tray == primary)
            {
                continue;
            }

            attempted++;
            var win = new NetSpeedTrayWidget();
            win.ApplyTheme(theme);
            win.ApplyAppearance(settings.NetSpeed);
            if (!win.TryEmbedOnTray(tray, out _))
            {
                failed++;
                win.Dispose();
                continue;
            }

            void Handler(object? s, RateSample sample) =>
                _ = _overlay.Dispatcher.InvokeAsync(() => win.ApplySample(sample));
            _sampler.Sampled += Handler;
            win.Disposed += (_, _) => _sampler.Sampled -= Handler;
            _secondaryOverlays.Add(win);
        }

        if (attempted == 0)
        {
            return;
        }

        if (failed == attempted)
        {
            ReportStatus("副屏网速钉不上，主屏不受影响。");
        }
        else if (failed > 0)
        {
            ReportStatus("有的屏幕没法钉网速，已跳过。");
        }
    }

    private void DisposeSecondaryOverlays()
    {
        foreach (NetSpeedTrayWidget win in _secondaryOverlays.ToArray())
        {
            try
            {
                win.HideEmbed();
                win.Dispose();
            }
            catch
            {
            }
        }

        _secondaryOverlays.Clear();
    }

    private static bool HotkeyEquals(HotkeyBinding a, HotkeyBinding b) =>
        a.Control == b.Control
        && a.Shift == b.Shift
        && a.Alt == b.Alt
        && a.Win == b.Win
        && a.VirtualKey == b.VirtualKey;
}
