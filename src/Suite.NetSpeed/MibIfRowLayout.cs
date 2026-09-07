namespace Suite.NetSpeed;

/// <summary>
/// MSVC x64 layout of MIB_IF_ROW2 from netioapi.h (Learn, 2026-09-05).
/// GetIfTable2 table header is ULONG NumEntries plus 4 bytes of alignment before the first row.
/// </summary>
internal static class MibIfRowLayout
{
    public const int RowSize = 1352;
    public const int TableHeaderSize = 8;
    public const int OffsetInterfaceIndex = 8;
    public const int OffsetAlias = 28;
    public const int OffsetDescription = 542;
    public const int OffsetType = 1128;
    public const int OffsetOperStatus = 1156;
    public const int OffsetMediaConnectState = 1164;
    public const int OffsetInOctets = 1208;
    public const int OffsetOutOctets = 1280;
    public const int IfOperStatusUp = 1;
    public const int MediaConnectStateConnected = 1;
}
