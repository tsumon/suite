using System.Runtime.InteropServices;

namespace Suite.NetSpeed;

public sealed class IpHelperInterfaceTable : IInterfaceTable
{
    public IReadOnlyList<InterfaceSnapshot> GetTable()
    {
        uint status = IpHelperNative.GetIfTable2(out IntPtr table);
        if (status != 0 || table == IntPtr.Zero)
        {
            throw new InvalidOperationException("GetIfTable2 failed with status " + status + ".");
        }

        try
        {
            int count = Marshal.ReadInt32(table);
            if (count < 0)
            {
                return Array.Empty<InterfaceSnapshot>();
            }

            var list = new List<InterfaceSnapshot>(count);
            for (int i = 0; i < count; i++)
            {
                IntPtr row = IntPtr.Add(table, MibIfRowLayout.TableHeaderSize + (i * MibIfRowLayout.RowSize));
                list.Add(ReadRow(row));
            }

            return list;
        }
        finally
        {
            IpHelperNative.FreeMibTable(table);
        }
    }

    public InterfaceSnapshot? GetEntry(uint ifIndex)
    {
        IntPtr row = Marshal.AllocHGlobal(MibIfRowLayout.RowSize);
        try
        {
            for (int i = 0; i < MibIfRowLayout.RowSize; i++)
            {
                Marshal.WriteByte(row, i, 0);
            }

            Marshal.WriteInt32(row, MibIfRowLayout.OffsetInterfaceIndex, unchecked((int)ifIndex));
            uint status = IpHelperNative.GetIfEntry2(row);
            if (status != 0)
            {
                return null;
            }

            return ReadRow(row);
        }
        finally
        {
            Marshal.FreeHGlobal(row);
        }
    }

    private static InterfaceSnapshot ReadRow(IntPtr row)
    {
        uint index = unchecked((uint)Marshal.ReadInt32(row, MibIfRowLayout.OffsetInterfaceIndex));
        string alias = Marshal.PtrToStringUni(IntPtr.Add(row, MibIfRowLayout.OffsetAlias)) ?? "";
        string description = Marshal.PtrToStringUni(IntPtr.Add(row, MibIfRowLayout.OffsetDescription)) ?? "";
        uint type = unchecked((uint)Marshal.ReadInt32(row, MibIfRowLayout.OffsetType));
        int oper = Marshal.ReadInt32(row, MibIfRowLayout.OffsetOperStatus);
        int media = Marshal.ReadInt32(row, MibIfRowLayout.OffsetMediaConnectState);
        ulong inOctets = unchecked((ulong)Marshal.ReadInt64(row, MibIfRowLayout.OffsetInOctets));
        ulong outOctets = unchecked((ulong)Marshal.ReadInt64(row, MibIfRowLayout.OffsetOutOctets));
        return new InterfaceSnapshot
        {
            IfIndex = index,
            Alias = alias,
            Description = description,
            IfType = type,
            IsUp = oper == MibIfRowLayout.IfOperStatusUp,
            MediaConnected = media == MibIfRowLayout.MediaConnectStateConnected,
            InOctets = inOctets,
            OutOctets = outOctets,
        };
    }
}
