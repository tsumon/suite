namespace Suite.NetSpeed;

public interface IInterfaceTable
{
    IReadOnlyList<InterfaceSnapshot> GetTable();
    InterfaceSnapshot? GetEntry(uint ifIndex);
}
