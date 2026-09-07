namespace Suite.NetSpeed;

public static class AdapterResolver
{
    public static InterfaceSnapshot? Resolve(
        IReadOnlyList<InterfaceSnapshot> table,
        uint? preferredIfIndex,
        string? preferredAlias)
    {
        if (preferredIfIndex is uint idx)
        {
            InterfaceSnapshot? byIndex = table.FirstOrDefault(x => x.IfIndex == idx);
            if (byIndex is not null && IsUsablePreferred(byIndex))
            {
                return byIndex;
            }
        }

        if (!string.IsNullOrWhiteSpace(preferredAlias))
        {
            InterfaceSnapshot? byAlias = table.FirstOrDefault(
                x => string.Equals(x.Alias, preferredAlias, StringComparison.OrdinalIgnoreCase));
            if (byAlias is not null && IsUsablePreferred(byAlias))
            {
                return byAlias;
            }
        }

        return PickDefault(table);
    }

    /// <summary>
    /// Preferred NIC is kept only when it can actually carry traffic.
    /// Stale ifIndex (gone / down / loopback) must fall through to PickDefault — never stick on a dead virtual.
    /// </summary>
    public static bool IsUsablePreferred(InterfaceSnapshot x) =>
        x.IsUp && !x.IsLoopback && !x.IsTunnel;

    public static InterfaceSnapshot? PickDefault(IReadOnlyList<InterfaceSnapshot> table)
    {
        static int Score(InterfaceSnapshot x)
        {
            int score = 0;
            if (x.IsUp)
            {
                score += 100;
            }

            if (x.MediaConnected)
            {
                score += 50;
            }

            if (!LooksVirtual(x))
            {
                score += 40;
            }

            // IF_TYPE_ETHERNET_CSMACD=6, IF_TYPE_IEEE80211=71
            if (x.IfType == 6)
            {
                score += 20;
            }
            else if (x.IfType == 71)
            {
                score += 15;
            }

            if (!x.IsLoopback && !x.IsTunnel)
            {
                score += 5;
            }

            // Prefer adapters that have already seen traffic (physical uplink usually wins over idle VMware).
            ulong octets = x.InOctets + x.OutOctets;
            if (octets > 1_000_000UL)
            {
                score += 10;
            }
            else if (octets > 10_000UL)
            {
                score += 4;
            }

            return score;
        }

        return table
            .Where(x => !x.IsLoopback && !x.IsTunnel)
            .OrderByDescending(Score)
            .ThenByDescending(x => x.InOctets + x.OutOctets)
            .ThenBy(x => x.IfIndex)
            .FirstOrDefault();
    }

    public static bool LooksVirtual(InterfaceSnapshot x)
    {
        string hay = (x.Description ?? "") + " " + (x.Alias ?? "");
        return hay.Contains("VMware", StringComparison.OrdinalIgnoreCase)
            || hay.Contains("VirtualBox", StringComparison.OrdinalIgnoreCase)
            || hay.Contains("Hyper-V", StringComparison.OrdinalIgnoreCase)
            || hay.Contains("Wi-Fi Direct", StringComparison.OrdinalIgnoreCase)
            || hay.Contains("Bluetooth", StringComparison.OrdinalIgnoreCase)
            || hay.Contains("Loopback", StringComparison.OrdinalIgnoreCase)
            || hay.Contains("Virtual Adapter", StringComparison.OrdinalIgnoreCase)
            || hay.Contains("vEthernet", StringComparison.OrdinalIgnoreCase);
    }
}
