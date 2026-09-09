using System.ComponentModel;
using System.Globalization;
using System.Windows;
using System.Windows.Media;
using System.Windows.Threading;
using Suite.Contracts;
using Suite.NetSpeed;
using Suite.Platform;
using MediaColor = System.Windows.Media.Color;

namespace Suite.App;

public sealed class AdapterOption
{
    public uint IfIndex { get; init; }
    public string Alias { get; init; } = "";
    public string Display { get; init; } = "";
}

public partial class SettingsWindow : Window
{
    private const int SliderDebounceMs = 200;

    private readonly AppController _controller;
    private readonly DispatcherTimer _applyTimer;
    private bool _loading;
    private uint? _downColorArgb;
    private uint? _upColorArgb;
    private uint? _backgroundArgb;

    public SettingsWindow(AppController controller)
    {
        _controller = controller;
        _loading = true;
        _applyTimer = new DispatcherTimer
        {
            Interval = TimeSpan.FromMilliseconds(SliderDebounceMs),
        };
        _applyTimer.Tick += (_, _) =>
        {
            _applyTimer.Stop();
            FlushApply();
        };
        InitializeComponent();
        ApplyChromeFromSystem();
        PopulateKeys();
        PopulateTaskbarModes();
        PopulateUpdateChannels();
        PopulateColorPickKeys();
        PopulateScrollCaptureKeys();
        ApplyAboutVersion();
        LoadFrom(_controller.Settings);
        RefreshAdapters();
        UpdateCascades();
        _loading = false;
        RefreshLightTransparencyTip();
        SelectNav("general");
    }

    private void ApplyAboutVersion()
    {
        string ver = UpdateChecker.LocalVersionDisplay;
        Title = "Suite 设置 — " + ver;
        if (AboutVersionText is not null)
        {
            AboutVersionText.Text = "当前版本 " + ver;
        }
    }


    public void SetHotkeyConflict(string message, IReadOnlyList<HotkeyBinding> suggestions)
    {
        SetStatus(message);
        if (HotkeyConflictRow is null || HotkeySuggestText is null)
        {
            return;
        }

        HotkeyConflictRow.Visibility = Visibility.Visible;
        string alts = HotkeyCopy.FormatSuggestions(suggestions.Select(s => s.ToDisplayString()).ToList());
        HotkeySuggestText.Text = alts.Length == 0
            ? HotkeyCopy.NoFreeKey
            : "建议：" + alts;
    }

    public void ClearHotkeyConflict()
    {
        if (HotkeyConflictRow is not null)
        {
            HotkeyConflictRow.Visibility = Visibility.Collapsed;
        }

        if (HotkeySuggestText is not null)
        {
            HotkeySuggestText.Text = "";
        }
    }

    private void OnTryNextHotkey(object sender, RoutedEventArgs e)
    {
        if (_controller.TryApplyNextAvailableHotkey(out string status))
        {
            SetStatus(status);
            ClearHotkeyConflict();
            LoadFrom(_controller.Settings);
            return;
        }

        SetStatus(status);
        HotkeyConflictRow.Visibility = Visibility.Visible;
        HotkeySuggestText.Text = status;
    }

    /// <summary>DESIGN.md surface/ink/line flip. Call on load and after theme toggle.</summary>
    public void ApplyTheme(ThemeKind theme) => ApplyChrome(theme == ThemeKind.Light);

    public void ApplyChromeFromSystem()
    {
        bool light = true;
        if (PersonalizeRegistry.TryRead(out PersonalizeDwords values, out _))
        {
            light = values.AppsUseLight;
        }

        ApplyChrome(light);
    }

    private void ApplyChrome(bool light)
    {
        // SETTINGS-PAGE-POLISH v2.1 tokens (override flatter v2 geometry/tiers).
        MediaColor bg = light ? MediaColor.FromRgb(0xF3, 0xF3, 0xF3) : MediaColor.FromRgb(0x2B, 0x2B, 0x2B);
        MediaColor navBg = light ? MediaColor.FromRgb(0xE4, 0xE4, 0xE4) : MediaColor.FromRgb(0x1F, 0x1F, 0x1F);
        MediaColor navHover = light ? MediaColor.FromRgb(0xE0, 0xE0, 0xE0) : MediaColor.FromRgb(0x33, 0x33, 0x33);
        MediaColor navActive = light ? MediaColor.FromRgb(0xFF, 0xFF, 0xFF) : MediaColor.FromRgb(0x3A, 0x3A, 0x3A);
        MediaColor navActiveBorder = light ? MediaColor.FromRgb(0xE0, 0xE0, 0xE0) : MediaColor.FromRgb(0x3A, 0x3A, 0x3A);
        MediaColor navAccent = light ? MediaColor.FromRgb(0x17, 0x17, 0x17) : MediaColor.FromRgb(0xF5, 0xF5, 0xF5);
        MediaColor surface = light ? MediaColor.FromRgb(0xFA, 0xFA, 0xFA) : MediaColor.FromRgb(0x2B, 0x2B, 0x2B);
        MediaColor footer = light ? MediaColor.FromRgb(0xF3, 0xF3, 0xF3) : MediaColor.FromRgb(0x25, 0x25, 0x25);
        MediaColor ink = light ? MediaColor.FromRgb(0x17, 0x17, 0x17) : MediaColor.FromRgb(0xF5, 0xF5, 0xF5);
        MediaColor inkSecondary = light ? MediaColor.FromRgb(0x5C, 0x5C, 0x5C) : MediaColor.FromRgb(0xD4, 0xD4, 0xD4);
        MediaColor muted = light ? MediaColor.FromRgb(0x5A, 0x5A, 0x5A) : MediaColor.FromRgb(0xB0, 0xB0, 0xB0);
        MediaColor disabled = light ? MediaColor.FromRgb(0x9A, 0x9A, 0x9A) : MediaColor.FromRgb(0x8A, 0x8A, 0x8A);
        MediaColor line = light ? MediaColor.FromRgb(0xC8, 0xC8, 0xC8) : MediaColor.FromRgb(0x55, 0x55, 0x55);
        MediaColor status = muted;
        MediaColor statusErr = light ? MediaColor.FromRgb(0xB4, 0x23, 0x18) : MediaColor.FromRgb(0xFF, 0x8A, 0x80);

        Resources["SetBgBrush"] = new SolidColorBrush(bg);
        Resources["SetNavBgBrush"] = new SolidColorBrush(navBg);
        Resources["SetNavHoverBrush"] = new SolidColorBrush(navHover);
        Resources["SetNavActiveBrush"] = new SolidColorBrush(navActive);
        Resources["SetNavActiveBorderBrush"] = new SolidColorBrush(navActiveBorder);
        Resources["SetNavAccentBrush"] = new SolidColorBrush(navAccent);
        Resources["SetSurfaceBrush"] = new SolidColorBrush(surface);
        Resources["SetFooterBgBrush"] = new SolidColorBrush(footer);
        Resources["SurfaceBrush"] = new SolidColorBrush(surface);
        Resources["InkBrush"] = new SolidColorBrush(ink);
        Resources["InkSecondaryBrush"] = new SolidColorBrush(inkSecondary);
        Resources["MutedBrush"] = new SolidColorBrush(muted);
        Resources["DisabledBrush"] = new SolidColorBrush(disabled);
        Resources["LineBrush"] = new SolidColorBrush(line);
        Resources["StatusBrush"] = new SolidColorBrush(status);
        Resources["StatusErrBrush"] = new SolidColorBrush(statusErr);
        Background = (Brush)Resources["SetBgBrush"];
        Foreground = (Brush)Resources["InkBrush"];
        if (DownColorSwatch is not null)
        {
            UpdateColorSwatches();
        }

        // Re-apply nav selection chrome after token swap.
        if (NavGeneral is not null)
        {
            SelectNav(_currentNav);
        }
    }

    private string _currentNav = "general";

    private void OnNavClick(object sender, RoutedEventArgs e)
    {
        if (sender is System.Windows.Controls.Button btn && btn.Tag is string tag)
        {
            SelectNav(tag);
        }
    }

    private void SelectNav(string tag)
    {
        _currentNav = tag ?? "general";
        SetPageVisible(PageGeneral, _currentNav == "general");
        SetPageVisible(PageCapture, _currentNav == "capture");
        SetPageVisible(PageNetSpeed, _currentNav == "netspeed");
        SetPageVisible(PageTaskbar, _currentNav == "taskbar");
        SetPageVisible(PageAdvanced, _currentNav == "advanced");
        StyleNav(NavGeneral, _currentNav == "general");
        StyleNav(NavCapture, _currentNav == "capture");
        StyleNav(NavNetSpeed, _currentNav == "netspeed");
        StyleNav(NavTaskbar, _currentNav == "taskbar");
        StyleNav(NavAdvanced, _currentNav == "advanced");
    }

    private static void SetPageVisible(UIElement? page, bool visible)
    {
        if (page is null)
        {
            return;
        }

        page.Visibility = visible ? Visibility.Visible : Visibility.Collapsed;
    }

    private void StyleNav(System.Windows.Controls.Button? btn, bool active)
    {
        if (btn is null)
        {
            return;
        }

        // Pill fill + left 3×20 accent (template binds Accent to BorderBrush).
        btn.Background = active
            ? (Brush)Resources["SetNavActiveBrush"]
            : System.Windows.Media.Brushes.Transparent;
        btn.FontWeight = active ? FontWeights.SemiBold : FontWeights.Normal;
        btn.Foreground = active
            ? (Brush)Resources["InkBrush"]
            : (Brush)Resources["InkSecondaryBrush"];
        btn.BorderBrush = active
            ? (Brush)Resources["SetNavAccentBrush"]
            : System.Windows.Media.Brushes.Transparent;
    }

    public void SetStatus(string text)
    {
        StatusText.Text = text;
        bool err = !string.IsNullOrEmpty(text)
            && (text.Contains("失败", StringComparison.Ordinal)
                || text.Contains("无法", StringComparison.Ordinal)
                || text.Contains("没法", StringComparison.Ordinal)
                || text.Contains("至少显示", StringComparison.Ordinal));
        StatusText.Foreground = err
            ? (Brush)Resources["StatusErrBrush"]
            : (Brush)Resources["StatusBrush"];
    }

    public void SetTaskbarStatus(string text)
    {
        TaskbarFxStatus.Text = text;
        if (text.Contains("无法", StringComparison.Ordinal)
            || text.Contains("未交付", StringComparison.Ordinal)
            || text.Contains("拦住", StringComparison.Ordinal)
            || text.Contains("暂时找不到", StringComparison.Ordinal))
        {
            SetStatus(text);
        }
    }

    public void Reload()
    {
        LoadFrom(_controller.Settings);
        RefreshAdapters();
        UpdateCascades();
        RefreshLightTransparencyTip();
    }

    private void PopulateKeys()
    {
        for (int i = 1; i <= 24; i++)
        {
            KeyBox.Items.Add(new KeyOption(0x6F + i, "F" + i));
        }

        for (char c = 'A'; c <= 'Z'; c++)
        {
            KeyBox.Items.Add(new KeyOption(c, c.ToString()));
        }
    }

    private void LoadFrom(AppSettings settings)
    {
        _loading = true;
        StartWithWindowsBox.IsChecked = settings.StartWithWindows;
        NetSpeedVisibleBox.IsChecked = settings.NetSpeed.Visible;
        NetSpeedEmbedBox.IsChecked = settings.NetSpeed.EmbedInTaskbar;
        FontSizeSlider.Value = settings.NetSpeed.FontSizeDip;
        FontSizeLabel.Text = ((int)Math.Round(settings.NetSpeed.FontSizeDip)).ToString(CultureInfo.InvariantCulture);
        ShowDownloadBox.IsChecked = settings.NetSpeed.ShowDownload;
        ShowUploadBox.IsChecked = settings.NetSpeed.ShowUpload;
        _downColorArgb = settings.NetSpeed.DownColorArgb;
        _upColorArgb = settings.NetSpeed.UpColorArgb;
        _backgroundArgb = settings.NetSpeed.BackgroundArgb;
        BackgroundTransparentBox.IsChecked = settings.NetSpeed.BackgroundTransparent;
        ShowBorderBox.IsChecked = settings.NetSpeed.ShowBorder;
        TwoLineBox.IsChecked = settings.NetSpeed.TwoLine;
        BoldBox.IsChecked = settings.NetSpeed.Bold;
        UpdateColorSwatches();

        CtrlBox.IsChecked = settings.Hotkey.Control;
        ShiftBox.IsChecked = settings.Hotkey.Shift;
        AltBox.IsChecked = settings.Hotkey.Alt;
        WinBox.IsChecked = settings.Hotkey.Win;
        KeyOption? match = KeyBox.Items.OfType<KeyOption>()
            .FirstOrDefault(x => x.VirtualKey == settings.Hotkey.VirtualKey);
        KeyBox.SelectedItem = match ?? KeyBox.Items.OfType<KeyOption>().FirstOrDefault(x => x.VirtualKey == HotkeyBinding.DefaultVirtualKey);
        SaveFileBox.IsChecked = settings.Capture.SaveFileAfterCapture;
        PinAfterBox.IsChecked = settings.Capture.PinAfterCapture;
        SaveDirectoryBox.Text = settings.Capture.SaveDirectory;
        SaveDirectoryHint.Text = "空目录表示保存到：" + SettingsPaths.DefaultCaptureDirectory;
        if (ShowMagnifierBox is not null)
        {
            ShowMagnifierBox.IsChecked = settings.Capture.ShowMagnifier;
        }

        if (HistoryEnabledBox is not null)
        {
            HistoryEnabledBox.IsChecked = settings.Capture.HistoryEnabled;
        }

        if (HistoryPathHint is not null)
        {
            HistoryPathHint.Text = "历史目录：" + SettingsPaths.ResolveHistoryDirectory(settings.Capture.SaveDirectory);
        }

        if (ColorPickKeyBox is not null)
        {
            KeyOption? cp = ColorPickKeyBox.Items.OfType<KeyOption>()
                .FirstOrDefault(x => x.VirtualKey == settings.Capture.ColorPickHotkey.VirtualKey);
            ColorPickKeyBox.SelectedItem = cp ?? ColorPickKeyBox.Items.OfType<KeyOption>()
                .FirstOrDefault(x => x.VirtualKey == HotkeyBinding.DefaultColorPickVirtualKey);
        }

        if (ClickThroughHotkeyLabel is not null)
        {
            ClickThroughHotkeyLabel.Text = settings.Capture.PinClickThroughHotkey.ToDisplayString();
        }

        if (ScrollCaptureKeyBox is not null)
        {
            KeyOption? sc = ScrollCaptureKeyBox.Items.OfType<KeyOption>()
                .FirstOrDefault(x => x.VirtualKey == settings.Capture.ScrollCaptureHotkey.VirtualKey);
            ScrollCaptureKeyBox.SelectedItem = sc ?? ScrollCaptureKeyBox.Items.OfType<KeyOption>()
                .FirstOrDefault(x => x.VirtualKey == HotkeyBinding.DefaultScrollCaptureVirtualKey);
        }

        if (EmbedSecondaryBox is not null)
        {
            EmbedSecondaryBox.IsChecked = settings.NetSpeed.EmbedSecondary;
        }

        if (UpdateChannelBox is not null)
        {
            UpdateChannelBox.SelectedIndex = string.Equals(
                settings.Update.Channel, UpdateSettings.ChannelPreview, StringComparison.OrdinalIgnoreCase)
                ? 1
                : 0;
        }

        if (LoggingBox is not null)
        {
            LoggingBox.IsChecked = settings.Advanced.LoggingEnabled;
        }

        if (LoggingHelp is not null)
        {
            LoggingHelp.Text = "日志在 " + SettingsPaths.LogsDirectory + "。含一般操作，不含截图像素。";
        }

        if (LastStartLabel is not null)
        {
            LastStartLabel.Text = "上次启动：" + _controller.LastStartLabel;
        }

        TaskbarFxEnabledBox.IsChecked = settings.TaskbarFx.Enabled;
        SelectTaskbarMode(settings.TaskbarFx.Mode);
        SetArgbUi(settings.TaskbarFx.Argb);
        UpdateModeHelp();
        UpdateCascades();
        _loading = false;
    }

    private void RefreshAdapters()
    {
        _loading = true;
        IReadOnlyList<InterfaceSnapshot> adapters = _controller.ListAdapters();
        AdapterBox.Items.Clear();
        foreach (InterfaceSnapshot adapter in adapters)
        {
            string mark = adapter.IsUp ? "" : " [down]";
            AdapterBox.Items.Add(new AdapterOption
            {
                IfIndex = adapter.IfIndex,
                Alias = adapter.Alias,
                Display = $"{adapter.Alias} (IfIndex {adapter.IfIndex}){mark}",
            });
        }

        uint? selected = _controller.Settings.NetSpeed.IfIndex;
        AdapterOption? current = AdapterBox.Items.OfType<AdapterOption>()
            .FirstOrDefault(x => x.IfIndex == selected);
        AdapterBox.SelectedItem = current ?? AdapterBox.Items.OfType<AdapterOption>().FirstOrDefault();
        _loading = false;
    }

    private AppSettings ReadUi()
    {
        AppSettings settings = _controller.Settings.Clone();
        settings.StartWithWindows = StartWithWindowsBox.IsChecked == true;
        settings.NetSpeed.Visible = NetSpeedVisibleBox.IsChecked == true;
        settings.NetSpeed.EmbedInTaskbar = NetSpeedEmbedBox.IsChecked == true;
        settings.NetSpeed.FontSizeDip = FontSizeSlider.Value;
        settings.NetSpeed.ShowDownload = ShowDownloadBox.IsChecked == true;
        settings.NetSpeed.ShowUpload = ShowUploadBox.IsChecked == true;
        settings.NetSpeed.DownColorArgb = _downColorArgb;
        settings.NetSpeed.UpColorArgb = _upColorArgb;
        settings.NetSpeed.BackgroundTransparent = BackgroundTransparentBox.IsChecked == true;
        settings.NetSpeed.BackgroundArgb = _backgroundArgb;
        settings.NetSpeed.ShowBorder = ShowBorderBox.IsChecked == true;
        settings.NetSpeed.TwoLine = TwoLineBox.IsChecked == true;
        settings.NetSpeed.Bold = BoldBox.IsChecked == true;
        if (AdapterBox.SelectedItem is AdapterOption option)
        {
            settings.NetSpeed.IfIndex = option.IfIndex;
            settings.NetSpeed.AdapterAlias = option.Alias;
        }

        settings.Hotkey.Control = CtrlBox.IsChecked == true;
        settings.Hotkey.Shift = ShiftBox.IsChecked == true;
        settings.Hotkey.Alt = AltBox.IsChecked == true;
        settings.Hotkey.Win = WinBox.IsChecked == true;
        if (KeyBox.SelectedItem is KeyOption key)
        {
            settings.Hotkey.VirtualKey = key.VirtualKey;
        }

        settings.Capture.SaveFileAfterCapture = SaveFileBox.IsChecked == true;
        settings.Capture.PinAfterCapture = PinAfterBox.IsChecked == true;
        settings.Capture.SaveDirectory = (SaveDirectoryBox.Text ?? "").Trim();
        if (ShowMagnifierBox is not null)
        {
            settings.Capture.ShowMagnifier = ShowMagnifierBox.IsChecked == true;
        }

        if (HistoryEnabledBox is not null)
        {
            settings.Capture.HistoryEnabled = HistoryEnabledBox.IsChecked == true;
        }

        if (ColorPickKeyBox?.SelectedItem is KeyOption cpKey)
        {
            settings.Capture.ColorPickHotkey = new HotkeyBinding { VirtualKey = cpKey.VirtualKey };
        }

        if (ScrollCaptureKeyBox?.SelectedItem is KeyOption scKey)
        {
            settings.Capture.ScrollCaptureHotkey = new HotkeyBinding { VirtualKey = scKey.VirtualKey };
        }

        if (EmbedSecondaryBox is not null)
        {
            settings.NetSpeed.EmbedSecondary = EmbedSecondaryBox.IsChecked == true;
        }

        if (UpdateChannelBox is not null)
        {
            settings.Update.Channel = UpdateChannelBox.SelectedIndex == 1
                ? UpdateSettings.ChannelPreview
                : UpdateSettings.ChannelStable;
        }

        if (LoggingBox is not null)
        {
            settings.Advanced.LoggingEnabled = LoggingBox.IsChecked == true;
        }

        settings.TaskbarFx.Enabled = TaskbarFxEnabledBox.IsChecked == true;
        if (TaskbarModeBox.SelectedItem is ModeOption mode)
        {
            settings.TaskbarFx.Mode = mode.Wire;
        }

        settings.TaskbarFx.Argb = ParseArgb(TaskbarArgbBox.Text, settings.TaskbarFx.Argb);
        return settings;
    }

    /// <summary>Queue apply+save. Sliders debounce ~200ms; other controls apply immediately.</summary>
    private void ScheduleApply(bool debounce = false)
    {
        if (_loading)
        {
            return;
        }

        if (!debounce)
        {
            _applyTimer.Stop();
            FlushApply();
            return;
        }

        _applyTimer.Stop();
        _applyTimer.Start();
    }

    private void FlushApply()
    {
        if (_loading)
        {
            return;
        }

        AppSettings next = ReadUi();
        if (!next.NetSpeed.ShowDownload && !next.NetSpeed.ShowUpload)
        {
            SetStatus(NetSpeedCopy.AtLeastOneLine);
            return;
        }

        next.NetSpeed.Normalize();
        // Optimistic success; ApplySettings overwrites status on embed / hotkey / Run-key / taskbar fail.
        SetStatus("已保存");
        _controller.ApplySettings(next);
    }

    private void UpdateCascades()
    {
        bool saveFile = SaveFileBox.IsChecked == true;
        SetDimmed(SaveDirectoryRow, saveFile);
        if (SaveDirectoryHint is not null)
        {
            // Dedicated disabled ink — never Opacity hack (POLISH).
            SaveDirectoryHint.Foreground = saveFile
                ? (Brush)Resources["MutedBrush"]
                : (Brush)Resources["DisabledBrush"];
        }

        bool netSpeed = NetSpeedVisibleBox.IsChecked == true;
        SetDimmed(NetSpeedDetailsPanel, netSpeed);

        UpdateBackgroundRowEnabled();

        bool taskbarFx = TaskbarFxEnabledBox.IsChecked == true;
        SetDimmed(TaskbarFxDetailsPanel, taskbarFx);
        UpdateTaskbarAlphaEnabled();
        if (!_loading)
        {
            RefreshLightTransparencyTip();
        }
    }

    private void SetDimmed(UIElement element, bool enabled)
    {
        element.IsEnabled = enabled;
        if (element is FrameworkElement fe)
        {
            // Keep full opacity; cascade DisabledBrush onto text instead of dimming.
            fe.Opacity = 1;
        }

        ApplyDisabledInk(element, enabled);
    }

    private void ApplyDisabledInk(DependencyObject root, bool enabled)
    {
        Brush secondary = enabled
            ? (Brush)Resources["InkSecondaryBrush"]
            : (Brush)Resources["DisabledBrush"];
        Brush muted = enabled
            ? (Brush)Resources["MutedBrush"]
            : (Brush)Resources["DisabledBrush"];
        Brush primary = enabled
            ? (Brush)Resources["InkBrush"]
            : (Brush)Resources["DisabledBrush"];
        Style? helpStyle = TryFindResource("HelpText") as Style;
        Style? fieldStyle = TryFindResource("FieldLabel") as Style;
        Style? sectionStyle = TryFindResource("SectionTitle") as Style;

        foreach (object raw in LogicalTreeHelper.GetChildren(root))
        {
            if (raw is not DependencyObject child)
            {
                continue;
            }

            if (child is System.Windows.Controls.TextBlock tb)
            {
                if (helpStyle is not null && ReferenceEquals(tb.Style, helpStyle))
                {
                    tb.Foreground = muted;
                }
                else if ((fieldStyle is not null && ReferenceEquals(tb.Style, fieldStyle))
                         || (sectionStyle is not null && ReferenceEquals(tb.Style, sectionStyle)))
                {
                    tb.Foreground = primary;
                }
                else
                {
                    tb.Foreground = secondary;
                }
            }
            else if (child is System.Windows.Controls.CheckBox cb)
            {
                cb.Foreground = secondary;
            }

            ApplyDisabledInk(child, enabled);
        }
    }

    private void UpdateBackgroundRowEnabled()
    {
        bool enabled = BackgroundTransparentBox.IsChecked != true;
        SetDimmed(BackgroundColorRow, enabled);
    }

    private void UpdateTaskbarAlphaEnabled()
    {
        string wire = TaskbarModeBox.SelectedItem is ModeOption mode
            ? mode.Wire
            : TaskbarAppearanceModes.Acrylic;
        bool alphaUseful = string.Equals(wire, TaskbarAppearanceModes.Clear, StringComparison.OrdinalIgnoreCase)
            || string.Equals(wire, TaskbarAppearanceModes.Acrylic, StringComparison.OrdinalIgnoreCase);
        SetDimmed(TaskbarAlphaRow, alphaUseful);
    }

    private void OnSettingChanged(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        UpdateCascades();
        ScheduleApply();
    }

    private void OnSelectionChanged(object sender, System.Windows.Controls.SelectionChangedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        ScheduleApply();
    }

    private void OnTextLostFocus(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        ScheduleApply();
    }

    private void OnShowLineChanged(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        if (ShowDownloadBox.IsChecked != true && ShowUploadBox.IsChecked != true)
        {
            _loading = true;
            if (ReferenceEquals(sender, ShowDownloadBox))
            {
                ShowDownloadBox.IsChecked = true;
            }
            else
            {
                ShowUploadBox.IsChecked = true;
            }

            _loading = false;
            SetStatus(NetSpeedCopy.AtLeastOneLine);
            return;
        }

        ScheduleApply();
    }

    private void OnRefreshAdapters(object sender, RoutedEventArgs e)
    {
        RefreshAdapters();
        // Selection may change after rebuild; sync disk/alias without waiting for another click.
        ScheduleApply();
    }


    private void OnFontSizeChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (FontSizeLabel is null)
        {
            return;
        }

        FontSizeLabel.Text = ((int)Math.Round(FontSizeSlider.Value)).ToString(CultureInfo.InvariantCulture);
        if (_loading)
        {
            return;
        }

        ScheduleApply(debounce: true);
    }

    private void OnBackgroundTransparentChanged(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        UpdateBackgroundRowEnabled();
        ScheduleApply();
    }

    private void UpdateColorSwatches()
    {
        DownColorSwatch.Background = new SolidColorBrush(SwatchColor(_downColorArgb, fallbackInk: true));
        UpColorSwatch.Background = new SolidColorBrush(SwatchColor(_upColorArgb, fallbackInk: true));
        bool light = IsAppsLightTheme();
        BackgroundColorSwatch.Background = new SolidColorBrush(
            _backgroundArgb is uint bg
                ? ArgbToColor(bg)
                : (light ? MediaColor.FromRgb(0xF2, 0xF2, 0xF2) : MediaColor.FromRgb(0x2B, 0x2B, 0x2B)));
    }

    private MediaColor SwatchColor(uint? argb, bool fallbackInk)
    {
        if (argb is uint value)
        {
            return ArgbToColor(value);
        }

        if (!fallbackInk)
        {
            return IsAppsLightTheme()
                ? MediaColor.FromRgb(0xF2, 0xF2, 0xF2)
                : MediaColor.FromRgb(0x2B, 0x2B, 0x2B);
        }

        return IsAppsLightTheme()
            ? MediaColor.FromRgb(0x1A, 0x1A, 0x1A)
            : MediaColor.FromRgb(0xF0, 0xF0, 0xF0);
    }

    private static bool IsAppsLightTheme() =>
        !PersonalizeRegistry.TryRead(out PersonalizeDwords values, out _) || values.AppsUseLight;

    private static MediaColor ArgbToColor(uint argb) =>
        MediaColor.FromArgb(
            (byte)((argb >> 24) & 0xFF),
            (byte)((argb >> 16) & 0xFF),
            (byte)((argb >> 8) & 0xFF),
            (byte)(argb & 0xFF));

    private void OnPickDownColor(object sender, RoutedEventArgs e)
    {
        if (TryPickColor(_downColorArgb ?? 0xFF1A1A1Au, out uint argb))
        {
            _downColorArgb = argb | 0xFF000000u;
            UpdateColorSwatches();
            ScheduleApply();
        }
    }

    private void OnPickUpColor(object sender, RoutedEventArgs e)
    {
        if (TryPickColor(_upColorArgb ?? 0xFF1A1A1Au, out uint argb))
        {
            _upColorArgb = argb | 0xFF000000u;
            UpdateColorSwatches();
            ScheduleApply();
        }
    }

    private void OnResetDownColor(object sender, RoutedEventArgs e)
    {
        _downColorArgb = null;
        UpdateColorSwatches();
        ScheduleApply();
    }

    private void OnResetUpColor(object sender, RoutedEventArgs e)
    {
        _upColorArgb = null;
        UpdateColorSwatches();
        ScheduleApply();
    }

    private void OnPickBackgroundColor(object sender, RoutedEventArgs e)
    {
        if (TryPickColor(_backgroundArgb ?? 0xFF2B2B2Bu, out uint argb))
        {
            _backgroundArgb = argb | 0xFF000000u;
            UpdateColorSwatches();
            ScheduleApply();
        }
    }

    private static bool TryPickColor(uint currentArgb, out uint argb)
    {
        argb = currentArgb;
        using var dialog = new System.Windows.Forms.ColorDialog
        {
            FullOpen = true,
            Color = System.Drawing.Color.FromArgb(
                255,
                (int)((currentArgb >> 16) & 0xFF),
                (int)((currentArgb >> 8) & 0xFF),
                (int)(currentArgb & 0xFF)),
        };
        if (dialog.ShowDialog() != System.Windows.Forms.DialogResult.OK)
        {
            return false;
        }

        argb = 0xFF000000u
            | ((uint)dialog.Color.R << 16)
            | ((uint)dialog.Color.G << 8)
            | dialog.Color.B;
        return true;
    }


    private void OnClose(object sender, RoutedEventArgs e) => Close();

    protected override void OnClosing(CancelEventArgs e)
    {
        // 关闭 = close only. Flush only if a slider debounce is still pending.
        if (!_loading && _applyTimer.IsEnabled)
        {
            _applyTimer.Stop();
            FlushApply();
        }

        base.OnClosing(e);
    }


    public const string LightTransparencyHelp =
        "浅色主题下，透明任务栏上的网速可能看不清。可关「背景透明」、给下行/上行选深色，或把任务栏效果改成实色。";

    /// <summary>INTERACTION-P1 §4: inline tip only — never force-change Clear/Acrylic.</summary>
    public void RefreshLightTransparencyTip()
    {
        if (LightTransparencyTip is null)
        {
            return;
        }

        bool light = IsAppsLightTheme();
        bool taskbarFx = TaskbarFxEnabledBox.IsChecked == true;
        string wire = TaskbarModeBox.SelectedItem is ModeOption mode
            ? mode.Wire
            : TaskbarAppearanceModes.Acrylic;
        bool clearOrAcrylic = string.Equals(wire, TaskbarAppearanceModes.Clear, StringComparison.OrdinalIgnoreCase)
            || string.Equals(wire, TaskbarAppearanceModes.Acrylic, StringComparison.OrdinalIgnoreCase);
        bool netTransparent = BackgroundTransparentBox.IsChecked == true;
        bool show = light && ((taskbarFx && clearOrAcrylic) || netTransparent);
        LightTransparencyTip.Text = LightTransparencyHelp;
        LightTransparencyTip.Visibility = show ? Visibility.Visible : Visibility.Collapsed;
        if (UseAcrylicButton is not null)
        {
            UseAcrylicButton.Visibility = show ? Visibility.Visible : Visibility.Collapsed;
        }
    }

    private void OnRestoreDefaults(object sender, RoutedEventArgs e)
    {
        MessageBoxResult answer = System.Windows.MessageBox.Show(
            this,
            "网速外观、任务栏效果、截图选项等会回到默认。不会卸载 Suite，也不会删你已保存的截图文件。",
            "恢复默认设置？",
            MessageBoxButton.OKCancel,
            MessageBoxImage.Warning,
            MessageBoxResult.Cancel);
        if (answer != MessageBoxResult.OK)
        {
            return;
        }

        _controller.RestoreDefaults();
        LoadFrom(_controller.Settings);
        RefreshAdapters();
        UpdateCascades();
        RefreshLightTransparencyTip();
    }

    private void OnStartScrollCapture(object sender, RoutedEventArgs e)
    {
        _controller.StartScrollCapture();
    }

    private void OnToggleTheme(object sender, RoutedEventArgs e)
    {
        _controller.ToggleTheme();
        // Registry already flipped; refresh this window chrome even if broadcast lagged.
        ApplyChromeFromSystem();
        RefreshLightTransparencyTip();
    }


    private void PopulateUpdateChannels()
    {
        if (UpdateChannelBox is null)
        {
            return;
        }

        UpdateChannelBox.Items.Clear();
        UpdateChannelBox.Items.Add("正式");
        UpdateChannelBox.Items.Add("预览");
    }

    private void PopulateColorPickKeys()
    {
        if (ColorPickKeyBox is null)
        {
            return;
        }

        ColorPickKeyBox.Items.Clear();
        for (int i = 1; i <= 24; i++)
        {
            ColorPickKeyBox.Items.Add(new KeyOption(0x6F + i, "F" + i));
        }
    }

    private void PopulateScrollCaptureKeys()
    {
        if (ScrollCaptureKeyBox is null)
        {
            return;
        }

        ScrollCaptureKeyBox.Items.Clear();
        ScrollCaptureKeyBox.Items.Add(new KeyOption(0, "关闭（无热键）"));
        for (int i = 1; i <= 24; i++)
        {
            ScrollCaptureKeyBox.Items.Add(new KeyOption(0x6F + i, "F" + i));
        }
    }

    private void OnOpenHistory(object sender, RoutedEventArgs e) => _controller.ShowHistory();

    private void OnCheckUpdates(object sender, RoutedEventArgs e) => _ = _controller.CheckUpdatesAsync();

    private void OnUseAcrylic(object sender, RoutedEventArgs e)
    {
        if (_controller.TrySwitchToAcrylicReadable(out string status))
        {
            SetStatus(status);
            LoadFrom(_controller.Settings);
            RefreshLightTransparencyTip();
            return;
        }

        SetStatus(status);
    }

    private void OnLoggingChanged(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        if (LoggingBox.IsChecked == true && !SuiteLog.TryWriteProbe(out string? error))
        {
            _loading = true;
            LoggingBox.IsChecked = false;
            _loading = false;
            SetStatus(error ?? "没法写日志，已关闭开关。");
            return;
        }

        ScheduleApply();
    }

    private void OnOpenLogs(object sender, RoutedEventArgs e)
    {
        if (!SuiteLog.TryOpenFolder(out string? error))
        {
            SetStatus(error ?? "打不开日志文件夹。");
        }
    }

    private void PopulateTaskbarModes()
    {
        TaskbarModeBox.Items.Add(new ModeOption(TaskbarAppearanceModes.Normal, TaskbarFxCopy.ModeLabel(TaskbarAppearanceModes.Normal)));
        TaskbarModeBox.Items.Add(new ModeOption(TaskbarAppearanceModes.Opaque, TaskbarFxCopy.ModeLabel(TaskbarAppearanceModes.Opaque)));
        TaskbarModeBox.Items.Add(new ModeOption(TaskbarAppearanceModes.Clear, TaskbarFxCopy.ModeLabel(TaskbarAppearanceModes.Clear)));
        TaskbarModeBox.Items.Add(new ModeOption(TaskbarAppearanceModes.Acrylic, TaskbarFxCopy.ModeLabel(TaskbarAppearanceModes.Acrylic)));
    }

    private void SelectTaskbarMode(string mode)
    {
        ModeOption? match = TaskbarModeBox.Items.OfType<ModeOption>()
            .FirstOrDefault(x => string.Equals(x.Wire, mode, StringComparison.OrdinalIgnoreCase));
        TaskbarModeBox.SelectedItem = match
            ?? TaskbarModeBox.Items.OfType<ModeOption>()
                .FirstOrDefault(x => x.Wire == TaskbarAppearanceModes.Acrylic);
    }

    private void SetArgbUi(uint argb)
    {
        TaskbarArgbBox.Text = "#" + argb.ToString("X8", CultureInfo.InvariantCulture);
        byte alpha = (byte)((argb >> 24) & 0xFF);
        TaskbarAlphaSlider.Value = alpha;
        TaskbarAlphaLabel.Text = alpha.ToString(CultureInfo.InvariantCulture);
    }

    private void OnTaskbarAlphaChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (_loading)
        {
            return;
        }

        uint argb = ParseArgb(TaskbarArgbBox.Text, TaskbarFxSettings.DefaultArgb);
        byte alpha = (byte)Math.Clamp((int)TaskbarAlphaSlider.Value, 0, 255);
        argb = (argb & 0x00FFFFFFu) | ((uint)alpha << 24);
        TaskbarArgbBox.Text = "#" + argb.ToString("X8", CultureInfo.InvariantCulture);
        TaskbarAlphaLabel.Text = alpha.ToString(CultureInfo.InvariantCulture);
        ScheduleApply(debounce: true);
    }

    private void OnTaskbarModeChanged(object sender, System.Windows.Controls.SelectionChangedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        UpdateModeHelp();
        UpdateTaskbarAlphaEnabled();
        ScheduleApply();
    }

    private void UpdateModeHelp()
    {
        string wire = TaskbarModeBox.SelectedItem is ModeOption mode ? mode.Wire : TaskbarAppearanceModes.Acrylic;
        TaskbarModeHelp.Text = TaskbarFxCopy.ModeHelp(wire);
    }

    private void OnPickTaskbarColor(object sender, RoutedEventArgs e)
    {
        uint current = ParseArgb(TaskbarArgbBox.Text, TaskbarFxSettings.DefaultArgb);
        using var dialog = new System.Windows.Forms.ColorDialog
        {
            FullOpen = true,
            Color = System.Drawing.Color.FromArgb(
                255,
                (int)((current >> 16) & 0xFF),
                (int)((current >> 8) & 0xFF),
                (int)(current & 0xFF)),
        };
        if (dialog.ShowDialog() != System.Windows.Forms.DialogResult.OK)
        {
            return;
        }

        byte alpha = (byte)Math.Clamp((int)TaskbarAlphaSlider.Value, 0, 255);
        uint argb = ((uint)alpha << 24)
            | ((uint)dialog.Color.R << 16)
            | ((uint)dialog.Color.G << 8)
            | dialog.Color.B;
        SetArgbUi(argb);
        ScheduleApply();
    }

    private static uint ParseArgb(string? text, uint fallback)
    {
        string raw = (text ?? "").Trim();
        if (raw.StartsWith('#') || raw.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
        {
            raw = raw[0] == '#' ? raw[1..] : raw[2..];
        }

        if (raw.Length == 6
            && uint.TryParse(raw, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out uint rgb))
        {
            return 0xFF000000u | rgb;
        }

        if (raw.Length == 8
            && uint.TryParse(raw, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out uint argb))
        {
            return argb;
        }

        return fallback;
    }

    private sealed record KeyOption(int VirtualKey, string Name)
    {
        public override string ToString() => Name;
    }

    private sealed record ModeOption(string Wire, string Label)
    {
        public override string ToString() => Label;
    }
}
