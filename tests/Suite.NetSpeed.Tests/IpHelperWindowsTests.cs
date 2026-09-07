using Suite.NetSpeed;

namespace Suite.NetSpeed.Tests;

public sealed class IpHelperWindowsTests
{
    [Fact]
    public void GetIfTable2_enumerates_or_is_skipped_off_windows()
    {
        if (!OperatingSystem.IsWindows())
        {
            return; // 须 Windows：Debian 上本项目是 net10.0-windows，不会跑到这里。
        }

        var table = new IpHelperInterfaceTable();
        IReadOnlyList<InterfaceSnapshot> rows = table.GetTable();
        Assert.NotNull(rows);
        foreach (InterfaceSnapshot row in rows)
        {
            Assert.False(string.IsNullOrWhiteSpace(row.Alias) && row.IfIndex == 0);
        }
    }
}
