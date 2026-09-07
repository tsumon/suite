using System.IO;
using Microsoft.Win32;

namespace Suite.Platform;

/// <summary>
/// HKCU Run only. Reversible. Uninstall must delete the Suite value.
/// </summary>
public static class RunKeyService
{
    public static bool IsEnabled()
    {
        using var key = Registry.CurrentUser.OpenSubKey(NativeConstants.RunSubKey, writable: false);
        return key?.GetValue(NativeConstants.RunValueName) is string s && !string.IsNullOrWhiteSpace(s);
    }

    public static bool TrySetEnabled(bool enabled, string executablePath, out string? error)
    {
        error = null;
        try
        {
            using var key = Registry.CurrentUser.CreateSubKey(NativeConstants.RunSubKey, writable: true)
                ?? throw new InvalidOperationException("Unable to open HKCU Run key.");
            if (enabled)
            {
                if (string.IsNullOrWhiteSpace(executablePath) || !File.Exists(executablePath))
                {
                    error = "Executable path is missing; cannot write the Run key.";
                    return false;
                }

                key.SetValue(NativeConstants.RunValueName, Quote(executablePath), RegistryValueKind.String);
            }
            else
            {
                key.DeleteValue(NativeConstants.RunValueName, throwOnMissingValue: false);
            }

            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    private static string Quote(string path) =>
        path.Contains(' ', StringComparison.Ordinal) && !path.StartsWith('"')
            ? "\"" + path + "\""
            : path;
}
