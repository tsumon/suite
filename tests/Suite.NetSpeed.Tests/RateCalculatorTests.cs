using Suite.NetSpeed;

namespace Suite.NetSpeed.Tests;

public sealed class RateCalculatorTests
{
    [Fact]
    public void Computes_bytes_per_second_from_octet_delta()
    {
        var previous = Row(12, 1_000, 2_000);
        var current = Row(12, 1_000 + (1024 * 1024), 2_000 + 2048);
        Assert.True(RateCalculator.TryCompute(previous, current, 1.0, out double rx, out double tx));
        Assert.Equal(1024 * 1024, rx, 3);
        Assert.Equal(2048, tx, 3);
    }

    [Fact]
    public void Rejects_zero_elapsed()
    {
        var row = Row(1, 10, 10);
        Assert.False(RateCalculator.TryCompute(row, row, 0, out _, out _));
    }

    [Fact]
    public void Counter_decrease_is_treated_as_reset()
    {
        var previous = Row(1, 5000, 5000);
        var current = Row(1, 10, 10);
        Assert.False(RateCalculator.TryCompute(previous, current, 1, out double rx, out double tx));
        Assert.Equal(0, rx);
        Assert.Equal(0, tx);
    }

    [Fact]
    public void Different_ifindex_is_rejected()
    {
        Assert.False(RateCalculator.TryCompute(Row(1, 0, 0), Row(2, 100, 100), 1, out _, out _));
    }

    [Fact]
    public void Format_uses_kb_and_mb()
    {
        Assert.Equal("512 B/s", RateFormatter.FormatBytesPerSecond(512));
        Assert.Equal("1.5 KB/s", RateFormatter.FormatBytesPerSecond(1536));
        Assert.Equal("2.00 MB/s", RateFormatter.FormatBytesPerSecond(2 * 1024 * 1024));
    }

    private static InterfaceSnapshot Row(uint ifIndex, ulong inbound, ulong outbound) => new()
    {
        IfIndex = ifIndex,
        Alias = "eth",
        InOctets = inbound,
        OutOctets = outbound,
        IsUp = true,
    };
}
