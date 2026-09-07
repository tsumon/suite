using System.Runtime.InteropServices;

namespace Suite.NetSpeed;

internal static class IpHelperNative
{
    [DllImport("iphlpapi.dll")]
    internal static extern uint GetIfTable2(out IntPtr table);

    [DllImport("iphlpapi.dll")]
    internal static extern uint GetIfEntry2(IntPtr row);

    [DllImport("iphlpapi.dll")]
    internal static extern void FreeMibTable(IntPtr table);
}
