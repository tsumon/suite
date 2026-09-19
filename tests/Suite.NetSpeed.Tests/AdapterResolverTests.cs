using Suite.NetSpeed;

namespace Suite.NetSpeed.Tests;

public sealed class AdapterResolverTests
{
    [Fact]
    public void Prefers_ifindex_when_still_present_and_up()
    {
        var table = new[] { Up("Wi-Fi", 3), Up("Ethernet", 12) };
        InterfaceSnapshot? hit = AdapterResolver.Resolve(table, 12, "Wi-Fi");
        Assert.NotNull(hit);
        Assert.Equal(12u, hit.IfIndex);
    }

    [Fact]
    public void Falls_back_when_preferred_ifindex_is_down()
    {
        var table = new[]
        {
            new InterfaceSnapshot { IfIndex = 9, Alias = "本地连接* 8", IfType = 6, IsUp = false, MediaConnected = false },
            Up("Ethernet", 20, inOctets: 5_000_000),
        };
        InterfaceSnapshot? hit = AdapterResolver.Resolve(table, 9, "本地连接* 8");
        Assert.NotNull(hit);
        Assert.Equal(20u, hit.IfIndex);
    }

    [Fact]
    public void Falls_back_to_alias_when_ifindex_gone()
    {
        var table = new[] { Up("Ethernet", 99) };
        InterfaceSnapshot? hit = AdapterResolver.Resolve(table, 12, "Ethernet");
        Assert.NotNull(hit);
        Assert.Equal(99u, hit.IfIndex);
    }

    [Fact]
    public void Default_skips_loopback_and_picks_up_ethernet()
    {
        var table = new[]
        {
            new InterfaceSnapshot { IfIndex = 1, Alias = "Loopback", IfType = 24, IsUp = true, MediaConnected = true },
            new InterfaceSnapshot { IfIndex = 8, Alias = "Ethernet", IfType = 6, IsUp = true, MediaConnected = true, InOctets = 100 },
            new InterfaceSnapshot { IfIndex = 9, Alias = "Down", IfType = 6, IsUp = false, MediaConnected = false },
        };

        InterfaceSnapshot? hit = AdapterResolver.Resolve(table, 1, "missing");
        Assert.NotNull(hit);
        Assert.Equal("Ethernet", hit.Alias);
    }

    [Fact]
    public void Default_prefers_physical_ethernet_over_vmware_and_wifi_direct()
    {
        var table = new[]
        {
            new InterfaceSnapshot
            {
                IfIndex = 4,
                Alias = "VMware Network Adapter VMnet8",
                Description = "VMware Virtual Ethernet Adapter for VMnet8",
                IfType = 6,
                IsUp = true,
                MediaConnected = true,
                InOctets = 1000,
            },
            new InterfaceSnapshot
            {
                IfIndex = 10,
                Alias = "本地连接* 10",
                Description = "Microsoft Wi-Fi Direct Virtual Adapter #2",
                IfType = 6,
                IsUp = true,
                MediaConnected = true,
                InOctets = 50_000,
            },
            new InterfaceSnapshot
            {
                IfIndex = 20,
                Alias = "以太网",
                Description = "Intel(R) Ethernet Controller (3) I225-V",
                IfType = 6,
                IsUp = true,
                MediaConnected = true,
                InOctets = 1_200_000_000,
                OutOctets = 100_000_000,
            },
        };

        InterfaceSnapshot? hit = AdapterResolver.PickDefault(table);
        Assert.NotNull(hit);
        Assert.Equal(20u, hit.IfIndex);
    }

    [Fact]
    public void Missing_everything_returns_null()
    {
        Assert.Null(AdapterResolver.Resolve(Array.Empty<InterfaceSnapshot>(), 1, "x"));
    }

    [Fact]
    public void Automatic_selection_moves_to_adapter_with_new_traffic()
    {
        var previous = new[]
        {
            Up("Ethernet", 12, inOctets: 10_000),
            Up("Wi-Fi", 3, inOctets: 20_000),
        };
        var current = new[]
        {
            Up("Ethernet", 12, inOctets: 10_000),
            Up("Wi-Fi", 3, inOctets: 25_000),
        };

        InterfaceSnapshot? hit = AdapterResolver.Resolve(current, 12, "Ethernet", previous);

        Assert.NotNull(hit);
        Assert.Equal(3u, hit.IfIndex);
    }

    [Fact]
    public void Automatic_selection_keeps_preferred_adapter_when_it_has_traffic()
    {
        var previous = new[]
        {
            Up("Ethernet", 12, inOctets: 10_000),
            Up("Wi-Fi", 3, inOctets: 20_000),
        };
        var current = new[]
        {
            Up("Ethernet", 12, inOctets: 12_000),
            Up("Wi-Fi", 3, inOctets: 25_000),
        };

        InterfaceSnapshot? hit = AdapterResolver.Resolve(current, 12, "Ethernet", previous);

        Assert.NotNull(hit);
        Assert.Equal(12u, hit.IfIndex);
    }

    private static InterfaceSnapshot Up(string alias, uint ifIndex, ulong inOctets = 0) => new()
    {
        Alias = alias,
        IfIndex = ifIndex,
        IfType = 6,
        IsUp = true,
        MediaConnected = true,
        InOctets = inOctets,
    };
}
