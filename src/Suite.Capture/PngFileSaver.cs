using System.IO;
using System.Windows.Media.Imaging;
using Suite.Contracts;

namespace Suite.Capture;

public static class PngFileSaver
{
    public static string Save(PixelBuffer buffer, string? configuredDirectory)
    {
        string directory = SettingsPaths.ResolveCaptureDirectory(configuredDirectory);
        Directory.CreateDirectory(directory);
        string name = "Suite-" + DateTime.Now.ToString("yyyyMMdd-HHmmss") + ".png";
        string path = Path.Combine(directory, name);
        if (File.Exists(path))
        {
            path = Path.Combine(directory, "Suite-" + DateTime.Now.ToString("yyyyMMdd-HHmmss-fff") + ".png");
        }

        string temp = path + ".tmp";
        try
        {
            var encoder = new PngBitmapEncoder();
            encoder.Frames.Add(BitmapFrame.Create(buffer.ToBitmapSource()));
            using (var stream = new FileStream(temp, FileMode.Create, FileAccess.Write, FileShare.None))
            {
                encoder.Save(stream);
            }

            File.Move(temp, path, overwrite: false);
            return path;
        }
        catch
        {
            TryDelete(temp);
            throw;
        }
    }

    private static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }
        }
        catch
        {
        }
    }
}
