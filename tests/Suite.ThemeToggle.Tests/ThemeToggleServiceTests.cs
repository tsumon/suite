using Suite.Contracts;
using Suite.ThemeToggle;

namespace Suite.ThemeToggle.Tests;

public sealed class ThemeToggleServiceTests
{
    [Fact]
    public void Toggle_light_to_dark_writes_both_dwords_and_broadcasts()
    {
        var store = new FakePersonalizeStore { Apps = ThemeKind.Light, System = ThemeKind.Light };
        var broadcast = new FakeThemeBroadcaster();
        var service = new ThemeToggleService(store, broadcast);
        ThemeKind? observed = null;
        service.ThemeChanged += (_, e) => observed = e.Theme;

        ThemeToggleResult result = service.Toggle();

        Assert.True(result.Succeeded);
        Assert.True(result.BroadcastSucceeded);
        Assert.Equal(ThemeKind.Dark, result.NewTheme);
        Assert.Equal(ThemeKind.Dark, store.LastWrittenApps);
        Assert.Equal(ThemeKind.Dark, store.LastWrittenSystem);
        Assert.Equal(1, broadcast.BroadcastCount);
        Assert.Equal(ThemeKind.Dark, observed);
    }

    [Fact]
    public void Toggle_dark_to_light()
    {
        var store = new FakePersonalizeStore { Apps = ThemeKind.Dark, System = ThemeKind.Dark };
        var broadcast = new FakeThemeBroadcaster();
        var service = new ThemeToggleService(store, broadcast);

        ThemeToggleResult result = service.Toggle();

        Assert.True(result.Succeeded);
        Assert.Equal(ThemeKind.Light, result.NewTheme);
        Assert.Equal(ThemeKind.Light, store.Apps);
        Assert.Equal(ThemeKind.Light, store.System);
    }

    [Fact]
    public void Write_failure_does_not_broadcast_or_raise()
    {
        var store = new FakePersonalizeStore { WriteShouldFail = true };
        var broadcast = new FakeThemeBroadcaster();
        var service = new ThemeToggleService(store, broadcast);
        bool raised = false;
        service.ThemeChanged += (_, _) => raised = true;

        ThemeToggleResult result = service.Toggle();

        Assert.False(result.Succeeded);
        Assert.Equal(0, broadcast.BroadcastCount);
        Assert.False(raised);
        Assert.Contains("policy", result.Error, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Read_failure_does_not_write()
    {
        var store = new FakePersonalizeStore { ReadShouldFail = true };
        var broadcast = new FakeThemeBroadcaster();
        var service = new ThemeToggleService(store, broadcast);

        ThemeToggleResult result = service.Toggle();

        Assert.False(result.Succeeded);
        Assert.Equal(0, store.WriteCount);
        Assert.Equal(0, broadcast.BroadcastCount);
    }

    [Fact]
    public void Broadcast_failure_still_raises_local_theme_changed()
    {
        var store = new FakePersonalizeStore();
        var broadcast = new FakeThemeBroadcaster { ShouldFail = true };
        var service = new ThemeToggleService(store, broadcast);
        bool raised = false;
        service.ThemeChanged += (_, _) => raised = true;

        ThemeToggleResult result = service.Toggle();

        Assert.True(result.Succeeded);
        Assert.False(result.BroadcastSucceeded);
        Assert.True(raised);
        Assert.Equal(ThemeKind.Dark, store.System);
    }

    [Fact]
    public void Split_apps_and_system_are_unified_from_system_value()
    {
        var store = new FakePersonalizeStore { Apps = ThemeKind.Light, System = ThemeKind.Dark };
        var broadcast = new FakeThemeBroadcaster();
        var service = new ThemeToggleService(store, broadcast);

        ThemeToggleResult result = service.Toggle();

        Assert.True(result.Succeeded);
        Assert.Equal(ThemeKind.Light, result.NewTheme);
        Assert.Equal(ThemeKind.Light, store.LastWrittenApps);
        Assert.Equal(ThemeKind.Light, store.LastWrittenSystem);
    }
}
