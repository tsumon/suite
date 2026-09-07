namespace Suite.NetSpeed;

public sealed class RateSample
{
    public required uint IfIndex { get; init; }
    public required string Alias { get; init; }
    public required double ReceiveBytesPerSecond { get; init; }
    public required double SendBytesPerSecond { get; init; }
    public bool AdapterMissing { get; init; }
}
