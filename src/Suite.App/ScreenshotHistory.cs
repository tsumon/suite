using System.IO;
using System.Windows;
using System.Windows.Media.Imaging;
using Suite.Contracts;

namespace Suite.App;

/// <summary>Recent captures under History\\. INTERACTION-P2 §3. Memory: files on disk, no retained bitmaps.</summary>
public static class ScreenshotHistory
{
    public const string Empty = "还没有截图记录。";
    public const string OpenFail = "打不开截图历史。";
    public const string Missing = "这张图找不到了。";
    public const string PinFail = "没钉上，请再试一次。";
    public const string Copied = "已复制。";
    public const string Cleared = "已清空历史。";
    public const int MaxAgeDays = 7;

    public sealed record Entry(string Path, DateTime LastWriteUtc);

    public static string DirectoryFor(AppSettings settings) =>
        SettingsPaths.ResolveHistoryDirectory(settings.Capture.SaveDirectory);

    public static IReadOnlyList<Entry> List(AppSettings settings)
    {
        string dir = DirectoryFor(settings);
        if (!Directory.Exists(dir))
        {
            return Array.Empty<Entry>();
        }

        DateTime cutoff = DateTime.UtcNow.AddDays(-MaxAgeDays);
        int max = Math.Clamp(settings.Capture.HistoryMax, 1, 200);
        try
        {
            return Directory.EnumerateFiles(dir, "*.png", SearchOption.TopDirectoryOnly)
                .Select(p => new Entry(p, File.GetLastWriteTimeUtc(p)))
                .Where(e => e.LastWriteUtc >= cutoff)
                .OrderByDescending(e => e.LastWriteUtc)
                .Take(max)
                .ToList();
        }
        catch
        {
            return Array.Empty<Entry>();
        }
    }

    /// <summary>Copy PNG from BitmapSource; prune excess. Does not keep the bitmap.</summary>
    public static string? TryAdd(BitmapSource image, AppSettings settings)
    {
        if (!settings.Capture.HistoryEnabled || image is null)
        {
            return null;
        }

        try
        {
            string dir = DirectoryFor(settings);
            Directory.CreateDirectory(dir);
            string name = "hist-" + DateTime.Now.ToString("yyyyMMdd-HHmmss-fff") + ".png";
            string path = Path.Combine(dir, name);
            string temp = path + ".tmp";
            var encoder = new PngBitmapEncoder();
            BitmapSource src = image;
            if (src.CanFreeze && !src.IsFrozen)
            {
                src = src.Clone();
                src.Freeze();
            }

            encoder.Frames.Add(BitmapFrame.Create(src));
            using (var stream = new FileStream(temp, FileMode.Create, FileAccess.Write, FileShare.None))
            {
                encoder.Save(stream);
            }

            File.Move(temp, path, overwrite: false);
            Prune(settings);
            return path;
        }
        catch
        {
            return null;
        }
    }

    public static void Prune(AppSettings settings)
    {
        string dir = DirectoryFor(settings);
        if (!Directory.Exists(dir))
        {
            return;
        }

        DateTime cutoff = DateTime.UtcNow.AddDays(-MaxAgeDays);
        int max = Math.Clamp(settings.Capture.HistoryMax, 1, 200);
        try
        {
            var files = Directory.EnumerateFiles(dir, "*.png", SearchOption.TopDirectoryOnly)
                .Select(p => (Path: p, Time: File.GetLastWriteTimeUtc(p)))
                .OrderByDescending(x => x.Time)
                .ToList();
            for (int i = 0; i < files.Count; i++)
            {
                if (i >= max || files[i].Time < cutoff)
                {
                    TryDelete(files[i].Path);
                }
            }
        }
        catch
        {
        }
    }

    public static bool TryDelete(string path)
    {
        try
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }

            return true;
        }
        catch
        {
            return false;
        }
    }

    public static bool ClearAll(AppSettings settings)
    {
        string dir = DirectoryFor(settings);
        if (!Directory.Exists(dir))
        {
            return true;
        }

        bool ok = true;
        foreach (string f in Directory.EnumerateFiles(dir, "*.png"))
        {
            if (!TryDelete(f))
            {
                ok = false;
            }
        }

        return ok;
    }

    public static bool TryLoadBitmap(string path, out BitmapSource? image, out string? error)
    {
        image = null;
        error = null;
        try
        {
            if (!File.Exists(path))
            {
                error = Missing;
                return false;
            }

            var bi = new BitmapImage();
            bi.BeginInit();
            bi.CacheOption = BitmapCacheOption.OnLoad; // release file handle; no lingering decode stream
            bi.UriSource = new Uri(path, UriKind.Absolute);
            bi.EndInit();
            bi.Freeze();
            image = bi;
            return true;
        }
        catch
        {
            error = Missing;
            return false;
        }
    }

    public static bool TryCopyFileToClipboard(string path, out string? error)
    {
        error = null;
        if (!TryLoadBitmap(path, out BitmapSource? image, out error) || image is null)
        {
            return false;
        }

        try
        {
            Clipboard.SetImage(image);
            return true;
        }
        catch
        {
            error = "无法写入剪贴板。";
            return false;
        }
    }
}
