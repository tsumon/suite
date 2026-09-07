using System.Globalization;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media.Imaging;
using Suite.Contracts;

namespace Suite.App;

public partial class HistoryWindow : Window
{
    private readonly AppController _controller;

    public HistoryWindow(AppController controller)
    {
        _controller = controller;
        InitializeComponent();
        PathHint.Text = "目录：" + ScreenshotHistory.DirectoryFor(_controller.Settings);
        Closed += (_, _) => ClearThumbs();
        Reload();
    }

    private void ClearThumbs()
    {
        if (List.ItemsSource is IEnumerable<Row> old)
        {
            foreach (Row r in old)
            {
                r.Thumb = null;
            }
        }

        List.ItemsSource = null;
    }

    private void Reload()
    {
        // Dispose prior thumbs to avoid pinning decoded bitmaps.
        if (List.ItemsSource is IEnumerable<Row> old)
        {
            foreach (Row r in old)
            {
                r.Thumb = null;
            }
        }

        var rows = new List<Row>();
        foreach (ScreenshotHistory.Entry e in ScreenshotHistory.List(_controller.Settings))
        {
            BitmapImage? thumb = null;
            try
            {
                thumb = new BitmapImage();
                thumb.BeginInit();
                thumb.CacheOption = BitmapCacheOption.OnLoad;
                thumb.DecodePixelWidth = 64;
                thumb.UriSource = new Uri(e.Path, UriKind.Absolute);
                thumb.EndInit();
                thumb.Freeze();
            }
            catch
            {
                thumb = null;
            }

            rows.Add(new Row
            {
                Path = e.Path,
                TimeLabel = e.LastWriteUtc.ToLocalTime().ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.CurrentCulture),
                Thumb = thumb,
            });
        }

        List.ItemsSource = rows;
        EmptyText.Visibility = rows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        List.Visibility = rows.Count == 0 ? Visibility.Collapsed : Visibility.Visible;
    }

    private void OnCopy(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string path })
        {
            return;
        }

        if (ScreenshotHistory.TryCopyFileToClipboard(path, out string? error))
        {
            _controller.ReportUserStatus(ScreenshotHistory.Copied);
        }
        else
        {
            _controller.ReportUserStatus(error ?? ScreenshotHistory.Missing);
            Reload();
        }
    }

    private void OnPin(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string path })
        {
            return;
        }

        if (!_controller.TryPinHistory(path, out string? error))
        {
            _controller.ReportUserStatus(error ?? ScreenshotHistory.PinFail);
            Reload();
        }
    }

    private void OnDelete(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string path })
        {
            return;
        }

        ScreenshotHistory.TryDelete(path);
        Reload();
    }

    private void OnClear(object sender, RoutedEventArgs e)
    {
        MessageBoxResult answer = MessageBox.Show(
            this,
            "清空截图历史？",
            "截图历史",
            MessageBoxButton.OKCancel,
            MessageBoxImage.Warning,
            MessageBoxResult.Cancel);
        if (answer != MessageBoxResult.OK)
        {
            return;
        }

        ScreenshotHistory.ClearAll(_controller.Settings);
        _controller.ReportUserStatus(ScreenshotHistory.Cleared);
        Reload();
    }

    private void OnDoubleClick(object sender, System.Windows.Input.MouseButtonEventArgs e)
    {
        if (List.SelectedItem is Row row)
        {
            if (ScreenshotHistory.TryCopyFileToClipboard(row.Path, out string? error))
            {
                _controller.ReportUserStatus(ScreenshotHistory.Copied);
            }
            else
            {
                _controller.ReportUserStatus(error ?? ScreenshotHistory.Missing);
            }
        }
    }

    private void OnClose(object sender, RoutedEventArgs e) => Close();

    private sealed class Row
    {
        public string Path { get; init; } = "";
        public string TimeLabel { get; init; } = "";
        public BitmapSource? Thumb { get; set; }
    }
}
