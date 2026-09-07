namespace Suite.Capture;

/// <summary>
/// Overlay rules from design/CAPTURE-PIN-SPEC.md.
/// Default path: release opens toolbar. Auto-pin only when PinAfterCapture is on.
/// </summary>
public static class CaptureUx
{
    public const int DragThresholdPx = 4;
    public const int MinCommitPx = 4;
    /// <summary>Ignore Close / right-click / dim-click for this long after annotate Loaded (ms).</summary>
    public const int AnnotateArmMs = 400;
    public const string OverlayHint = "悬停点窗，拖动手选。松手出工具栏。Esc 取消。";

    public enum ReleaseAction
    {
        Stay,
        CommitHover,
        CommitDrag,
    }

    public static bool OpenAnnotationAfterRegion(bool pinAfterCapture, bool annotateModifier) =>
        annotateModifier || !pinAfterCapture;

    public static bool IsDrag(int startX, int startY, int x, int y)
    {
        int dx = x - startX;
        int dy = y - startY;
        return ((long)dx * dx) + ((long)dy * dy) >= (long)DragThresholdPx * DragThresholdPx;
    }

    public static ReleaseAction DecideRelease(bool isDragging, int dragWidth, int dragHeight, bool hasHover)
    {
        if (isDragging)
        {
            if (dragWidth >= MinCommitPx && dragHeight >= MinCommitPx)
            {
                return ReleaseAction.CommitDrag;
            }

            return hasHover ? ReleaseAction.CommitHover : ReleaseAction.Stay;
        }

        return hasHover ? ReleaseAction.CommitHover : ReleaseAction.Stay;
    }

    public static string SizeChip(int width, int height) => width + " × " + height + " px";

    /// <summary>
    /// 平移选区，保持宽高不变，并夹在 clampBounds 内（整框不越界）。
    /// </summary>
    public static PixelRect MoveRect(PixelRect origin, int dx, int dy, PixelRect clampBounds)
    {
        if (origin.IsEmpty || clampBounds.IsEmpty)
        {
            return origin;
        }

        int w = origin.Width;
        int h = origin.Height;
        int x = origin.X + dx;
        int y = origin.Y + dy;

        int minX = clampBounds.X;
        int minY = clampBounds.Y;
        int maxX = clampBounds.Right - w;
        int maxY = clampBounds.Bottom - h;
        if (maxX < minX)
        {
            // 选区比监视器并集还宽：贴左
            x = minX;
        }
        else
        {
            x = Math.Clamp(x, minX, maxX);
        }

        if (maxY < minY)
        {
            y = minY;
        }
        else
        {
            y = Math.Clamp(y, minY, maxY);
        }

        return new PixelRect(x, y, w, h);
    }

    /// <summary>监视器 Bounds 的并集；无帧时回退 fallback。</summary>
    public static PixelRect UnionMonitorBounds(IReadOnlyList<PixelRect> bounds, PixelRect fallback)
    {
        if (bounds.Count == 0)
        {
            return fallback;
        }

        int left = int.MaxValue;
        int top = int.MaxValue;
        int right = int.MinValue;
        int bottom = int.MinValue;
        foreach (PixelRect b in bounds)
        {
            if (b.IsEmpty)
            {
                continue;
            }

            left = Math.Min(left, b.X);
            top = Math.Min(top, b.Y);
            right = Math.Max(right, b.Right);
            bottom = Math.Max(bottom, b.Bottom);
        }

        if (right <= left || bottom <= top)
        {
            return fallback;
        }

        return new PixelRect(left, top, right - left, bottom - top);
    }

    /// <summary>
    /// Monitor index with the largest intersection area vs <paramref name="selection"/>.
    /// Ties keep the earlier index (stable). Returns -1 if none overlap.
    /// </summary>
    public static int IndexOfLargestOverlap(IReadOnlyList<PixelRect> monitorBounds, PixelRect selection)
    {
        if (monitorBounds.Count == 0 || selection.IsEmpty)
        {
            return -1;
        }

        int best = -1;
        long bestArea = -1;
        for (int i = 0; i < monitorBounds.Count; i++)
        {
            PixelRect overlap = selection.Intersect(monitorBounds[i]);
            if (overlap.IsEmpty)
            {
                continue;
            }

            long area = (long)overlap.Width * overlap.Height;
            if (area > bestArea)
            {
                bestArea = area;
                best = i;
            }
        }

        return best;
    }
}
