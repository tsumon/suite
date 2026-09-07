using System.Windows;
using System.Windows.Controls;
using Suite.Pinboard;

namespace Suite.App;

public partial class PinManageWindow : Window
{
    private readonly AppController _controller;

    public PinManageWindow(AppController controller)
    {
        _controller = controller;
        InitializeComponent();
        Reload();
    }

    private void Reload()
    {
        var rows = _controller.ListPins()
            .Select(p => new Row
            {
                Index = p.Index,
                Title = "贴图 " + p.Index,
                ThroughLabel = p.IsClickThrough ? "穿透关" : "穿透开",
            })
            .ToList();
        List.ItemsSource = rows;
        EmptyText.Visibility = rows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        List.Visibility = rows.Count == 0 ? Visibility.Collapsed : Visibility.Visible;
    }

    private void OnFocus(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: int index })
        {
            _controller.FocusPin(index);
        }
    }

    private void OnThrough(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: int index })
        {
            _controller.TogglePinThrough(index);
            Reload();
        }
    }

    private void OnCloseOne(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: int index })
        {
            _controller.ClosePin(index);
            Reload();
        }
    }

    private void OnCloseAll(object sender, RoutedEventArgs e)
    {
        MessageBoxResult answer = MessageBox.Show(
            this,
            "关闭全部贴图？",
            "贴图管理",
            MessageBoxButton.OKCancel,
            MessageBoxImage.Warning,
            MessageBoxResult.Cancel);
        if (answer != MessageBoxResult.OK)
        {
            return;
        }

        _controller.CloseAllPins();
        Reload();
    }

    private void OnClose(object sender, RoutedEventArgs e) => Close();

    private sealed class Row
    {
        public int Index { get; init; }
        public string Title { get; init; } = "";
        public string ThroughLabel { get; init; } = "";
    }
}
