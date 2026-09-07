using Microsoft.Win32;

namespace Suite.Platform;

public readonly record struct PersonalizeDwords(bool AppsUseLight, bool SystemUsesLight);

/// <summary>
/// HKCU Themes\Personalize. 1 = light, 0 = dark. Missing values treated as light.
/// </summary>
public static class PersonalizeRegistry
{
    public static bool TryRead(out PersonalizeDwords values, out string? error)
    {
        error = null;
        values = new PersonalizeDwords(true, true);
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(NativeConstants.PersonalizeSubKey, writable: false);
            bool apps = ReadDwordAsLight(key, NativeConstants.AppsUseLightTheme);
            bool system = ReadDwordAsLight(key, NativeConstants.SystemUsesLightTheme);
            values = new PersonalizeDwords(apps, system);
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    public static bool TryWrite(PersonalizeDwords values, out string? error)
    {
        error = null;
        try
        {
            using var key = Registry.CurrentUser.CreateSubKey(NativeConstants.PersonalizeSubKey, writable: true)
                ?? throw new InvalidOperationException("Unable to open HKCU Personalize key.");
            key.SetValue(NativeConstants.AppsUseLightTheme, values.AppsUseLight ? 1 : 0, RegistryValueKind.DWord);
            key.SetValue(NativeConstants.SystemUsesLightTheme, values.SystemUsesLight ? 1 : 0, RegistryValueKind.DWord);
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    private static bool ReadDwordAsLight(RegistryKey? key, string name)
    {
        if (key?.GetValue(name) is int i)
        {
            return i != 0;
        }

        if (key?.GetValue(name) is long l)
        {
            return l != 0;
        }

        return true;
    }
}
