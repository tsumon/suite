using Suite.ThemeToggle;

namespace Suite.ThemeToggle.Tests;

internal sealed class FakeThemeBroadcaster : IThemeBroadcaster
{
    public int BroadcastCount { get; private set; }
    public bool ShouldFail { get; set; }

    public bool TryBroadcast(out string? error)
    {
        BroadcastCount++;
        if (ShouldFail)
        {
            error = "broadcast timeout";
            return false;
        }

        error = null;
        return true;
    }
}
