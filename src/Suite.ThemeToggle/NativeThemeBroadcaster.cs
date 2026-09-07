using Suite.Platform;

namespace Suite.ThemeToggle;

public sealed class NativeThemeBroadcaster : IThemeBroadcaster
{
    public bool TryBroadcast(out string? error) => NativeBroadcast.SendImmersiveColorSet(out error);
}
