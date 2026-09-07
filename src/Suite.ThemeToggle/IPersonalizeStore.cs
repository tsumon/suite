using Suite.Contracts;

namespace Suite.ThemeToggle;

public interface IPersonalizeStore
{
    bool TryRead(out ThemeKind apps, out ThemeKind system, out string? error);
    bool TryWrite(ThemeKind apps, ThemeKind system, out string? error);
}
