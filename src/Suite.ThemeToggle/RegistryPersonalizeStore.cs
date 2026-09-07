using Suite.Contracts;
using Suite.Platform;

namespace Suite.ThemeToggle;

public sealed class RegistryPersonalizeStore : IPersonalizeStore
{
    public bool TryRead(out ThemeKind apps, out ThemeKind system, out string? error)
    {
        apps = ThemeKind.Light;
        system = ThemeKind.Light;
        if (!PersonalizeRegistry.TryRead(out var values, out error))
        {
            return false;
        }

        apps = values.AppsUseLight ? ThemeKind.Light : ThemeKind.Dark;
        system = values.SystemUsesLight ? ThemeKind.Light : ThemeKind.Dark;
        return true;
    }

    public bool TryWrite(ThemeKind apps, ThemeKind system, out string? error) =>
        PersonalizeRegistry.TryWrite(
            new PersonalizeDwords(apps == ThemeKind.Light, system == ThemeKind.Light),
            out error);
}
