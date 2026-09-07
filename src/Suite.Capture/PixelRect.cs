namespace Suite.Capture;

public readonly struct PixelRect : IEquatable<PixelRect>
{
    public PixelRect(int x, int y, int width, int height)
    {
        X = x;
        Y = y;
        Width = width;
        Height = height;
    }

    public int X { get; }
    public int Y { get; }
    public int Width { get; }
    public int Height { get; }
    public int Right => X + Width;
    public int Bottom => Y + Height;
    public bool IsEmpty => Width <= 0 || Height <= 0;

    public static PixelRect FromCorners(int x1, int y1, int x2, int y2)
    {
        int left = Math.Min(x1, x2);
        int top = Math.Min(y1, y2);
        int right = Math.Max(x1, x2);
        int bottom = Math.Max(y1, y2);
        return new PixelRect(left, top, right - left, bottom - top);
    }

    public PixelRect Intersect(PixelRect other)
    {
        int left = Math.Max(X, other.X);
        int top = Math.Max(Y, other.Y);
        int right = Math.Min(Right, other.Right);
        int bottom = Math.Min(Bottom, other.Bottom);
        if (right <= left || bottom <= top)
        {
            return default;
        }

        return new PixelRect(left, top, right - left, bottom - top);
    }

    public PixelRect ClampTo(int width, int height)
    {
        return Intersect(new PixelRect(0, 0, width, height));
    }

    public bool Contains(int x, int y) => x >= X && y >= Y && x < Right && y < Bottom;

    public bool Equals(PixelRect other) =>
        X == other.X && Y == other.Y && Width == other.Width && Height == other.Height;

    public override bool Equals(object? obj) => obj is PixelRect other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(X, Y, Width, Height);

    public override string ToString() => $"{X},{Y} {Width}x{Height}";
}
