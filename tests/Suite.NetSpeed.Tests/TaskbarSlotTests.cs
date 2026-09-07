using Suite.Platform;

namespace Suite.NetSpeed.Tests;

public sealed class TaskbarSlotTests
{
    [Fact]
    public void ComputeSlot_sits_left_of_notify_area()
    {
        var slot = TaskbarEmbed.ComputeSlot(trayClientWidth: 1920, trayClientHeight: 48, notifyLeft: 1800, desiredWidth: 148);
        Assert.Equal(1800 - 148 - 4, slot.X);
        Assert.Equal(0, slot.Y);
        Assert.Equal(148, slot.Width);
        Assert.Equal(48, slot.Height);
    }

    [Fact]
    public void ComputeSlot_falls_back_to_right_edge_without_notify()
    {
        var slot = TaskbarEmbed.ComputeSlot(trayClientWidth: 800, trayClientHeight: 40, notifyLeft: null, desiredWidth: 148);
        Assert.Equal(800 - 148 - 4, slot.X);
        Assert.Equal(40, slot.Height);
        Assert.Equal(148, slot.Width);
    }

    [Fact]
    public void ComputeSlot_clamps_when_tray_is_narrow()
    {
        var slot = TaskbarEmbed.ComputeSlot(trayClientWidth: 100, trayClientHeight: 40, notifyLeft: null, desiredWidth: 148);
        Assert.True(slot.Width <= 100);
        Assert.True(slot.X >= 0);
        Assert.True(slot.X + slot.Width <= 100);
    }

    [Fact]
    public void Slot_that_runs_into_notify_area_is_blocked()
    {
        Assert.True(TaskbarEmbed.SlotOverlapsNotify(slotX: 1700, slotWidth: 148, notifyLeft: 1800));
        Assert.False(TaskbarEmbed.SlotOverlapsNotify(slotX: 1648, slotWidth: 148, notifyLeft: 1800));
    }
}
