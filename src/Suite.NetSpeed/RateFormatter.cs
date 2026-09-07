using System.Globalization;

namespace Suite.NetSpeed;

public static class RateFormatter
{
    public static string FormatBytesPerSecond(double bytesPerSecond)
    {
        double abs = Math.Abs(bytesPerSecond);
        if (abs < 1024)
        {
            return string.Create(CultureInfo.InvariantCulture, $"{bytesPerSecond:0} B/s");
        }

        if (abs < 1024 * 1024)
        {
            return string.Create(CultureInfo.InvariantCulture, $"{bytesPerSecond / 1024:0.0} KB/s");
        }

        if (abs < 1024L * 1024 * 1024)
        {
            return string.Create(CultureInfo.InvariantCulture, $"{bytesPerSecond / (1024 * 1024):0.00} MB/s");
        }

        return string.Create(CultureInfo.InvariantCulture, $"{bytesPerSecond / (1024L * 1024 * 1024):0.00} GB/s");
    }
}
