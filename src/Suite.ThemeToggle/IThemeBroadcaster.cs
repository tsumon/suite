namespace Suite.ThemeToggle;

public interface IThemeBroadcaster
{
    bool TryBroadcast(out string? error);
}
