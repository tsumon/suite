namespace Suite.Capture;

internal static class RegionCropper
{
    public static PixelBuffer Crop(IReadOnlyList<MonitorCapture> frames, PixelRect virtualRect)
    {
        if (virtualRect.IsEmpty)
        {
            throw new ArgumentOutOfRangeException(nameof(virtualRect));
        }

        PixelBuffer dest = PixelBuffer.Allocate(virtualRect.Width, virtualRect.Height);
        foreach (MonitorCapture frame in frames)
        {
            PixelRect overlap = virtualRect.Intersect(frame.Monitor.Bounds);
            if (overlap.IsEmpty)
            {
                continue;
            }

            var srcRect = new PixelRect(
                Map(overlap.X - frame.Monitor.Bounds.X, frame.Monitor.Bounds.Width, frame.Buffer.Width),
                Map(overlap.Y - frame.Monitor.Bounds.Y, frame.Monitor.Bounds.Height, frame.Buffer.Height),
                Math.Max(1, Map(overlap.Width, frame.Monitor.Bounds.Width, frame.Buffer.Width)),
                Math.Max(1, Map(overlap.Height, frame.Monitor.Bounds.Height, frame.Buffer.Height)));
            dest.BlitFrom(frame.Buffer, srcRect, overlap.X - virtualRect.X, overlap.Y - virtualRect.Y);
        }

        return dest;
    }

    public static CaptureMethod DominantMethod(IReadOnlyList<MonitorCapture> frames, PixelRect virtualRect)
    {
        CaptureMethod best = CaptureMethod.None;
        int bestArea = -1;
        foreach (MonitorCapture frame in frames)
        {
            PixelRect overlap = virtualRect.Intersect(frame.Monitor.Bounds);
            int area = overlap.Width * overlap.Height;
            if (area > bestArea)
            {
                bestArea = area;
                best = frame.Method;
            }
        }

        return best;
    }

    private static int Map(int value, int from, int to)
    {
        if (from <= 0)
        {
            return 0;
        }

        return (int)Math.Round(value * (to / (double)from));
    }
}
