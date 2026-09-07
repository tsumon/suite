namespace Suite.Platform;

public static class NativeConstants
{
    public static readonly IntPtr HwndBroadcast = new(0xFFFF);
    public const uint WmSettingChange = 0x001A;
    public const uint WmHotKey = 0x0312;
    public const uint SmtoAbortIfHung = 0x0002;
    public const uint ImmersiveColorSetTimeoutMs = 300; // per-window if SendMessageTimeout; prefer async notify
    public const string ImmersiveColorSet = "ImmersiveColorSet";
    public const string RunSubKey = @"Software\Microsoft\Windows\CurrentVersion\Run";
    public const string RunValueName = "Suite";
    public const string PersonalizeSubKey = @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    public const string AppsUseLightTheme = "AppsUseLightTheme";
    public const string SystemUsesLightTheme = "SystemUsesLightTheme";
    public const string SingleInstanceMutexName = @"Local\Suite.App.SingleInstance";
    public const string TaskbarFxMutexName = @"Local\Suite.TaskbarFx.SingleInstance";
    public const string TaskbarFxPipePrefix = "Suite.TaskbarFx";
    public const string TaskbarFxExeFileName = "TaskbarFx.exe";
    public const string SuiteExeFileName = "Suite.exe";
    public const string TaskbarFxNativeFileName = "TaskbarFx.Native.dll";
    public const string TaskbarFxTapFileName = "TaskbarFx.Tap.dll";
    public const int PlaceholderHotKeyId = 1;
    public const int PinClipboardHotKeyId = 2;
    public const int ColorPickHotKeyId = 3;
    public const int PinClickThroughHotKeyId = 4;
}
