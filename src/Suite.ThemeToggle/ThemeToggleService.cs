using Suite.Contracts;

namespace Suite.ThemeToggle;

/// <summary>
/// Flip AppsUseLightTheme and SystemUsesLightTheme together, then broadcast ImmersiveColorSet.
/// Never terminates explorer. Registry write is synchronous; local ThemeChanged fires before broadcast
/// so UI chrome does not wait on HWND_BROADCAST. Broadcast itself is async notify (not per-window wait).
/// </summary>
public sealed class ThemeToggleService
{
    private readonly IPersonalizeStore _store;
    private readonly IThemeBroadcaster _broadcaster;

    public ThemeToggleService(IPersonalizeStore store, IThemeBroadcaster broadcaster)
    {
        _store = store;
        _broadcaster = broadcaster;
    }

    public event EventHandler<ThemeChangedEventArgs>? ThemeChanged;

    public bool TryRead(out ThemeKind theme, out string? error)
    {
        if (!_store.TryRead(out _, out ThemeKind system, out error))
        {
            theme = ThemeKind.Light;
            return false;
        }

        theme = system;
        error = null;
        return true;
    }

    public ThemeToggleResult Toggle()
    {
        if (!_store.TryRead(out _, out ThemeKind system, out string? readError))
        {
            return ThemeToggleResult.Fail(readError ?? "Unable to read Personalize values.", ThemeKind.Light);
        }

        ThemeKind next = system == ThemeKind.Light ? ThemeKind.Dark : ThemeKind.Light;
        if (!_store.TryWrite(next, next, out string? writeError))
        {
            return ThemeToggleResult.Fail(
                writeError ?? "Unable to write Personalize values. The key may be policy-locked.",
                system);
        }

        // Apply local chrome before broadcast so Settings/netspeed never wait on HWND_BROADCAST.
        ThemeChanged?.Invoke(this, new ThemeChangedEventArgs(next));

        bool broadcastOk = _broadcaster.TryBroadcast(out string? broadcastError);

        if (!broadcastOk)
        {
            return new ThemeToggleResult
            {
                Succeeded = true,
                BroadcastSucceeded = false,
                NewTheme = next,
                Error = broadcastError ?? "ImmersiveColorSet broadcast did not complete.",
            };
        }

        return new ThemeToggleResult
        {
            Succeeded = true,
            BroadcastSucceeded = true,
            NewTheme = next,
        };
    }
}
