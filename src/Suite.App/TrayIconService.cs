using System.Drawing;
using System.Windows.Forms;
using Suite.Contracts;

namespace Suite.App;

public sealed class TrayIconService : IDisposable
{
    private readonly NotifyIcon _icon;
    private readonly ToolStripMenuItem _themeItem;
    private readonly ToolStripMenuItem _netSpeedItem;
    private readonly ToolStripMenuItem _captureItem;
    private readonly ToolStripMenuItem _scrollCaptureItem;
    private readonly ToolStripMenuItem _colorPickItem;
    private readonly ToolStripMenuItem _historyItem;
    private readonly ToolStripMenuItem _pinItem;
    private readonly ToolStripMenuItem _pinManageItem;
    private readonly ToolStripMenuItem _clearThroughItem;
    private readonly ToolStripMenuItem _settingsItem;
    private readonly ToolStripMenuItem _runItem;
    private readonly ToolStripMenuItem _exitItem;
    private Icon? _ownedIcon;

    public TrayIconService()
    {
        _themeItem = new ToolStripMenuItem("切换系统深浅色");
        _netSpeedItem = new ToolStripMenuItem("显示网速");
        _captureItem = new ToolStripMenuItem("区域截图");
        _scrollCaptureItem = new ToolStripMenuItem("滚动长截图");
        _colorPickItem = new ToolStripMenuItem("取色");
        _historyItem = new ToolStripMenuItem("截图历史");
        _pinItem = new ToolStripMenuItem("贴图（从剪贴板）");
        _pinManageItem = new ToolStripMenuItem("贴图管理");
        _clearThroughItem = new ToolStripMenuItem("取消贴图穿透") { Enabled = false };
        _settingsItem = new ToolStripMenuItem("设置...");
        _runItem = new ToolStripMenuItem("开机启动") { CheckOnClick = true };
        _exitItem = new ToolStripMenuItem("退出");

        var menu = new ContextMenuStrip();
        menu.Items.Add(_themeItem);
        menu.Items.Add(_netSpeedItem);
        menu.Items.Add(_captureItem);
        menu.Items.Add(_scrollCaptureItem);
        menu.Items.Add(_colorPickItem);
        menu.Items.Add(_historyItem);
        menu.Items.Add(_pinItem);
        menu.Items.Add(_pinManageItem);
        menu.Items.Add(_clearThroughItem);
        menu.Items.Add(_settingsItem);
        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(_runItem);
        menu.Items.Add(_exitItem);

        _ownedIcon = CreateIcon();
        _icon = new NotifyIcon
        {
            Icon = _ownedIcon,
            Text = "Suite",
            Visible = true,
            ContextMenuStrip = menu,
        };
        _icon.DoubleClick += (_, _) => SettingsClicked?.Invoke(this, EventArgs.Empty);

        _themeItem.Click += (_, _) => ToggleThemeClicked?.Invoke(this, EventArgs.Empty);
        _netSpeedItem.Click += (_, _) => ToggleNetSpeedClicked?.Invoke(this, EventArgs.Empty);
        _captureItem.Click += (_, _) => CaptureClicked?.Invoke(this, EventArgs.Empty);
        _scrollCaptureItem.Click += (_, _) => ScrollCaptureClicked?.Invoke(this, EventArgs.Empty);
        _colorPickItem.Click += (_, _) => ColorPickClicked?.Invoke(this, EventArgs.Empty);
        _historyItem.Click += (_, _) => HistoryClicked?.Invoke(this, EventArgs.Empty);
        _pinItem.Click += (_, _) => PinFromClipboardClicked?.Invoke(this, EventArgs.Empty);
        _pinManageItem.Click += (_, _) => PinManageClicked?.Invoke(this, EventArgs.Empty);
        _clearThroughItem.Click += (_, _) => ClearClickThroughClicked?.Invoke(this, EventArgs.Empty);
        _settingsItem.Click += (_, _) => SettingsClicked?.Invoke(this, EventArgs.Empty);
        _runItem.CheckedChanged += OnRunCheckedChanged;
        _exitItem.Click += (_, _) => ExitClicked?.Invoke(this, EventArgs.Empty);
    }

    public event EventHandler? ToggleThemeClicked;
    public event EventHandler? ToggleNetSpeedClicked;
    public event EventHandler? CaptureClicked;
    public event EventHandler? ScrollCaptureClicked;
    public event EventHandler? ColorPickClicked;
    public event EventHandler? HistoryClicked;
    public event EventHandler? PinFromClipboardClicked;
    public event EventHandler? PinManageClicked;
    public event EventHandler? ClearClickThroughClicked;
    public event EventHandler? SettingsClicked;
    public event EventHandler<bool>? StartWithWindowsChanged;
    public event EventHandler? ExitClicked;

    public void Sync(AppSettings settings)
    {
        _netSpeedItem.Text = settings.NetSpeed.Visible ? "隐藏网速" : "显示网速";
        HotkeyBinding scroll = settings.Capture.ScrollCaptureHotkey;
        _scrollCaptureItem.Text = scroll.IsDisabled
            ? "滚动长截图"
            : "滚动长截图 (" + scroll.ToDisplayString() + ")";
        string tip = "Suite";
        if (!settings.Hotkey.IsDisabled)
        {
            tip += " · 截图 " + settings.Hotkey.ToDisplayString();
        }

        if (!scroll.IsDisabled)
        {
            tip += " · 滚动 " + scroll.ToDisplayString();
        }

        if (tip.Length > 63)
        {
            tip = tip[..63];
        }

        _icon.Text = tip;
        _runItem.CheckedChanged -= OnRunCheckedChanged;
        _runItem.Checked = settings.StartWithWindows;
        _runItem.CheckedChanged += OnRunCheckedChanged;
    }

    public void SetClickThroughAvailable(bool available) => _clearThroughItem.Enabled = available;

    public void Balloon(string title, string text)
    {
        _icon.BalloonTipTitle = title;
        _icon.BalloonTipText = text;
        _icon.ShowBalloonTip(4000);
    }

    public void Dispose()
    {
        _icon.Visible = false;
        _icon.Dispose();
        _ownedIcon?.Dispose();
        _ownedIcon = null;
    }

    private void OnRunCheckedChanged(object? sender, EventArgs e) =>
        StartWithWindowsChanged?.Invoke(this, _runItem.Checked);

    private static Icon CreateIcon()
    {
        var bmp = new Bitmap(16, 16);
        using (var g = Graphics.FromImage(bmp))
        {
            g.Clear(Color.FromArgb(36, 36, 40));
            using var fill = new SolidBrush(Color.White);
            g.FillRectangle(fill, 3, 3, 10, 10);
            using var inner = new SolidBrush(Color.FromArgb(36, 36, 40));
            g.FillRectangle(inner, 6, 6, 4, 4);
        }

        return Icon.FromHandle(bmp.GetHicon());
    }
}
