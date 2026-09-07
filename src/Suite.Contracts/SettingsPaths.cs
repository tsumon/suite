using System.IO;

namespace Suite.Contracts;

public static class SettingsPaths
{
    public const string AppFolderName = "Suite";
    public const string FileName = "settings.json";
    public const string CrashGuardFileName = "crash-guard.json";
    public const string HistoryFolderName = "History";
    public const string LogsFolderName = "logs";

    public static string DirectoryPath => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        AppFolderName);

    public static string FilePath => Path.Combine(DirectoryPath, FileName);

    public static string CrashGuardPath => Path.Combine(DirectoryPath, CrashGuardFileName);

    public static string CaptureLogPath => Path.Combine(DirectoryPath, "capture.log");
    public static string TaskbarFxLogPath => Path.Combine(DirectoryPath, "taskbarfx.log");

    public static string LogsDirectory => Path.Combine(DirectoryPath, LogsFolderName);

    public static string DefaultCaptureDirectory
    {
        get
        {
            string pictures = Environment.GetFolderPath(Environment.SpecialFolder.MyPictures);
            if (!string.IsNullOrWhiteSpace(pictures))
            {
                return Path.Combine(pictures, AppFolderName);
            }

            return Path.Combine(DirectoryPath, "Captures");
        }
    }

    public static string ResolveCaptureDirectory(string? configured)
    {
        if (!string.IsNullOrWhiteSpace(configured))
        {
            return configured.Trim();
        }

        return DefaultCaptureDirectory;
    }

    /// <summary>History lives under capture root History\ (INTERACTION-P2 §3.2).</summary>
    public static string ResolveHistoryDirectory(string? configuredCaptureDir) =>
        Path.Combine(ResolveCaptureDirectory(configuredCaptureDir), HistoryFolderName);
}
