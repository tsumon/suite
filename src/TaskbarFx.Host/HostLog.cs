using System.IO;
using Suite.Contracts;

namespace TaskbarFx.Host;

internal static class HostLog
{
    public static void Write(string line)
    {
        try
        {
            Directory.CreateDirectory(SettingsPaths.DirectoryPath);
            string stamp = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss");
            File.AppendAllText(SettingsPaths.TaskbarFxLogPath, stamp + " " + line + Environment.NewLine);
        }
        catch
        {
        }
    }
}
