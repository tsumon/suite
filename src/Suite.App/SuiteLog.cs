using System.IO;
using Suite.Contracts;

namespace Suite.App;

/// <summary>Optional debug log under LocalAppData\Suite\logs. No pixels. INTERACTION-P2 §11.</summary>
public static class SuiteLog
{
    private static readonly object Gate = new();
    private static bool _enabled;

    public static void SetEnabled(bool enabled)
    {
        lock (Gate)
        {
            _enabled = enabled;
        }
    }

    public static bool IsEnabled
    {
        get
        {
            lock (Gate)
            {
                return _enabled;
            }
        }
    }

    public static void Error(string message, Exception? ex = null)
    {
        string detail = ex is null ? message : message + " " + ex.GetType().Name + ": " + ex.Message;
        Info("ERROR " + detail);
        if (ex?.StackTrace is string st)
        {
            Info(st);
        }
    }

    public static void Info(string message)
    {
        if (!IsEnabled)
        {
            return;
        }

        try
        {
            Directory.CreateDirectory(SettingsPaths.LogsDirectory);
            string path = Path.Combine(SettingsPaths.LogsDirectory, "suite-" + DateTime.Now.ToString("yyyyMMdd") + ".log");
            string line = DateTime.Now.ToString("HH:mm:ss.fff") + " " + message + Environment.NewLine;
            lock (Gate)
            {
                File.AppendAllText(path, line);
            }
        }
        catch
        {
            // Caller may bounce the checkbox via WriteFailed.
        }
    }

    public static bool TryOpenFolder(out string? error)
    {
        error = null;
        try
        {
            Directory.CreateDirectory(SettingsPaths.LogsDirectory);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = SettingsPaths.LogsDirectory,
                UseShellExecute = true,
            });
            return true;
        }
        catch
        {
            error = "打不开日志文件夹。";
            return false;
        }
    }

    public static bool TryWriteProbe(out string? error)
    {
        error = null;
        try
        {
            Directory.CreateDirectory(SettingsPaths.LogsDirectory);
            string path = Path.Combine(SettingsPaths.LogsDirectory, "suite-" + DateTime.Now.ToString("yyyyMMdd") + ".log");
            File.AppendAllText(path, DateTime.Now.ToString("HH:mm:ss.fff") + " log-enabled" + Environment.NewLine);
            return true;
        }
        catch
        {
            error = "没法写日志，已关闭开关。";
            return false;
        }
    }
}
