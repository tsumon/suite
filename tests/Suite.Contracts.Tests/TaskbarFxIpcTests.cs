using Suite.Contracts;

namespace Suite.Contracts.Tests;

public sealed class TaskbarFxIpcTests
{
    [Theory]
    [InlineData("normal", TaskbarAppearanceMode.Normal)]
    [InlineData("opaque", TaskbarAppearanceMode.Opaque)]
    [InlineData("clear", TaskbarAppearanceMode.Clear)]
    [InlineData("acrylic", TaskbarAppearanceMode.Acrylic)]
    [InlineData("ACRYLIC", TaskbarAppearanceMode.Acrylic)]
    public void Mode_parse_roundtrip(string wire, TaskbarAppearanceMode expected)
    {
        Assert.True(TaskbarAppearanceModes.TryParse(wire, out TaskbarAppearanceMode mode));
        Assert.Equal(expected, mode);
        Assert.Equal(wire.ToLowerInvariant(), TaskbarAppearanceModes.ToWire(mode));
    }

    [Fact]
    public void Unknown_mode_is_rejected()
    {
        Assert.False(TaskbarAppearanceModes.TryParse("blur", out _));
    }

    [Fact]
    public void Envelope_roundtrip_preserves_ops_and_status()
    {
        TaskbarFxEnvelope original = TaskbarFxIpc.AppearanceRequest(TaskbarAppearanceModes.Opaque, 0xCC112233);
        original.Status = new TaskbarFxStatusDto
        {
            Enabled = true,
            Path = TaskbarFxIpc.PathWin10Swca,
            Mode = TaskbarAppearanceModes.Opaque,
            Argb = 0xCC112233,
            Message = "ok",
        };

        TaskbarFxEnvelope restored = TaskbarFxIpc.Deserialize(TaskbarFxIpc.Serialize(original));
        Assert.Equal(TaskbarFxIpc.OpSetAppearance, restored.Op);
        Assert.Equal(TaskbarAppearanceModes.Opaque, restored.Mode);
        Assert.Equal(0xCC112233u, restored.Argb);
        Assert.Equal(TaskbarFxIpc.PathWin10Swca, restored.Status?.Path);
        Assert.True(restored.Status?.Enabled);
    }

    [Fact]
    public async Task Frame_length_prefix_roundtrip()
    {
        await using var stream = new MemoryStream();
        const string json = "{\"v\":1,\"op\":\"Ping\"}";
        await TaskbarFxIpc.WriteFrameAsync(stream, json, CancellationToken.None);
        stream.Position = 0;
        string? read = await TaskbarFxIpc.ReadFrameAsync(stream, CancellationToken.None);
        Assert.Equal(json, read);
    }

    [Fact]
    public async Task Frame_rejects_oversize_length()
    {
        await using var stream = new MemoryStream();
        byte[] header = BitConverter.GetBytes(TaskbarFxIpc.MaxFrameBytes + 1);
        await stream.WriteAsync(header);
        stream.Position = 0;
        await Assert.ThrowsAsync<InvalidOperationException>(() =>
            TaskbarFxIpc.ReadFrameAsync(stream, CancellationToken.None));
    }

    [Fact]
    public void Call_timeout_is_two_seconds()
    {
        Assert.Equal(TimeSpan.FromSeconds(2), TaskbarFxIpc.CallTimeout);
    }

    [Fact]
    public void Protocol_ops_cover_minimum_set()
    {
        string[] ops =
        [
            TaskbarFxIpc.OpEnable,
            TaskbarFxIpc.OpDisable,
            TaskbarFxIpc.OpSetAppearance,
            TaskbarFxIpc.OpGetStatus,
            TaskbarFxIpc.OpShutdown,
            TaskbarFxIpc.OpPing,
            TaskbarFxIpc.OpEvent,
        ];
        Assert.Equal(7, ops.Distinct(StringComparer.Ordinal).Count());
    }

    [Fact]
    public void Invalid_taskbar_mode_in_settings_falls_back_to_acrylic()
    {
        AppSettings restored = SettingsJson.Deserialize("""{"taskbarFx":{"mode":"blur"}}""");
        Assert.Equal(TaskbarAppearanceModes.Acrylic, restored.TaskbarFx.Mode);
    }
}
