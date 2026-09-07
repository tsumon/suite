namespace Suite.NetSpeed;

public static class RateCalculator
{
    public static bool TryCompute(
        InterfaceSnapshot previous,
        InterfaceSnapshot current,
        double elapsedSeconds,
        out double receiveBytesPerSecond,
        out double sendBytesPerSecond)
    {
        receiveBytesPerSecond = 0;
        sendBytesPerSecond = 0;
        if (elapsedSeconds <= 0)
        {
            return false;
        }

        if (current.IfIndex != previous.IfIndex)
        {
            return false;
        }

        if (current.InOctets < previous.InOctets || current.OutOctets < previous.OutOctets)
        {
            return false;
        }

        receiveBytesPerSecond = (current.InOctets - previous.InOctets) / elapsedSeconds;
        sendBytesPerSecond = (current.OutOctets - previous.OutOctets) / elapsedSeconds;
        return true;
    }
}
