namespace Suite.NetSpeed;

public sealed class InterfaceSnapshot
{
    public uint IfIndex { get; init; }
    public string Alias { get; init; } = "";
    public string Description { get; init; } = "";
    public bool IsUp { get; init; }
    public uint IfType { get; init; }
    public bool MediaConnected { get; init; }
    public ulong InOctets { get; init; }
    public ulong OutOctets { get; init; }

    public bool IsLoopback => IfType == 24; // IF_TYPE_SOFTWARE_LOOPBACK
    public bool IsTunnel => IfType == 131; // IF_TYPE_TUNNEL
}
