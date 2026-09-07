using System.Globalization;
using System.Threading;
using Suite.Capture.Ocr;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;

namespace Suite.Capture;

public partial class AnnotationWindow : Window
{
    private static readonly Brush ToolbarBg = Freeze(Color.FromRgb(0xF2, 0xF2, 0xF2));
    private static readonly Brush ToolbarLine = Freeze(Color.FromRgb(0xC8, 0xC8, 0xC8));
    private static readonly Brush ToolbarHover = Freeze(Color.FromRgb(0xE8, 0xE8, 0xE8));
    private static readonly Brush ToolbarPressed = Freeze(Color.FromRgb(0xDD, 0xDD, 0xDD));
    private static readonly Brush SeparatorInk = Freeze(Color.FromRgb(0x7B, 0x7B, 0x7B));
    private static readonly Brush DotInk = Freeze(Color.FromRgb(0xFF, 0x00, 0x00));
    private static readonly Brush CommitInk = Freeze(Color.FromRgb(0x20, 0x80, 0xF0));
    private static readonly Brush AnchorFill = Freeze(Colors.White);
    private static readonly Brush ChipBg = Freeze(Color.FromArgb(0xCC, 0, 0, 0));
    private static readonly Brush DimBrush = Freeze(Color.FromArgb(0x6E, 0, 0, 0));
    private static readonly Brush SubBarBg = Freeze(Color.FromRgb(0x2B, 0x2B, 0x2B));
    private static readonly Brush SubBarChipBg = Freeze(Color.FromRgb(0x3A, 0x3A, 0x3A));
    private static readonly Brush SubBarChipOn = Freeze(Color.FromRgb(0x55, 0x55, 0x55));
    private static readonly Brush SubBarInk = Freeze(Color.FromRgb(0xEE, 0xEE, 0xEE));
    private static readonly Brush SubBarSep = Freeze(Color.FromRgb(0x66, 0x66, 0x66));

    private IReadOnlyList<MonitorCapture> _frames;
    private readonly MonitorInfo _monitor;
    private readonly IOcrService _ocr;
    private readonly Action<string>? _reportStatus;
    private bool _ocrBusy;
    private PixelBuffer? _cachedBake;
    private bool _bakeDirty = true;
    private readonly List<IAnnotationOp> _ops = [];
    private readonly List<IAnnotationOp> _redo = [];
    private readonly Dictionary<AnnotateTool, ToolButton> _buttons = [];
    private readonly Ellipse[] _anchors = new Ellipse[8];
    private readonly double _dipX;
    private readonly double _dipY;
    private PixelBuffer _original;
    private PixelRect _selection;
    private AnnotateTool? _tool;
    private bool _ellipse;
    private CurveStyle _curveStyle = CurveStyle.Solid;
    private bool _arrowAtEnd = true;
    private byte _strokeB = PixelDraw.StrokeB;
    private byte _strokeG = PixelDraw.StrokeG;
    private byte _strokeR = PixelDraw.StrokeR;
    private int _strokeThickness = 4;
    private int _markerThickness = 14;
    private int _mosaicRadius = 16;
    private int _eraserThickness = 18;
    private int _textSize = 18;
    private Polyline? _arrowHeadDraft;
    private PixelBuffer? _mosaicDraft;
    private int _mosaicDraftLast = -1;
    private readonly Border _statusChip = new();
    private readonly TextBlock _statusText = new();
    private CancellationTokenSource? _statusHideCts;
    private bool _dragging;
    private bool _resizing;
    private bool _moving;
    private Handle _handle = Handle.None;
    private Point _startDip;
    private List<(int X, int Y)>? _stroke;
    private Shape? _draft;
    private PixelRect _resizeOrigin;
    private PixelRect _moveOrigin;
    private (int X, int Y) _moveStartVirtual;

    private readonly Grid _imageHost = new();
    private readonly Image _preview = new();
    private readonly Canvas _draftLayer = new() { IsHitTestVisible = false };
    private readonly Canvas _inputLayer = new();
    private readonly TextBox _textEntry = new();
    private readonly Rectangle _border = new();
    private readonly Border _toolbar = new();
    private readonly Border _subBar = new();
    private double _cachedSubBarW = 120;
    private readonly Border _sizeChip = new();
    private readonly TextBlock _sizeText = new();
    private readonly Rectangle _dimTop = new();
    private readonly Rectangle _dimBottom = new();
    private readonly Rectangle _dimLeft = new();
    private readonly Rectangle _dimRight = new();
    private DateTime _armedUtc = DateTime.MaxValue;
    private ToolButton? _undoButton;
    private ToolButton? _redoButton;
    // Cached Measure results for drag/resize light layout (skip Measure every move).
    private double _cachedBarW = 120;
    private double _cachedChipW = 48;
    private double _cachedChipH = 18;
    private string? _cachedChipText;

    // Live move/resize: RenderTransform + rubber-band — avoid Image Measure/Arrange every mousemove.
    private readonly TranslateTransform _liveTx = new();
    private bool _liveTxAttached;
    private bool _renderHooked;
    private bool _dimsDirty;
    private PixelRect _rubberSelection;
    private double _laidSelL;
    private double _laidSelT;
    private double _laidSelW;
    private double _laidSelH;

    // Full-monitor frozen screenshot (Snipaste-like). Dim hole reveals live viewport during drag.
    private readonly Image _freezeImage = new();
    private BitmapSource? _freezeSource;
    /// <summary>Reused preview surface — mosaic stamps / RefreshPreview must not allocate a new frozen BitmapSource each move.</summary>
    private WriteableBitmap? _previewBmp;
    private bool _liveViewport;

    internal AnnotationWindow(
        IReadOnlyList<MonitorCapture> frames,
        PixelRect selection,
        MonitorInfo monitor,
        IOcrService? ocr = null,
        Action<string>? reportStatus = null)
    {
        _frames = frames;
        _monitor = monitor;
        _ocr = ocr ?? new NullOcrService();
        _reportStatus = reportStatus;
        Focusable = true;
        Loaded += (_, _) => ArmKeyboardFocus();
        ContentRendered += (_, _) => ArmKeyboardFocus();
        Activated += (_, _) => ArmKeyboardFocus();
        _selection = selection;
        _original = RegionCropper.Crop(frames, selection);
        _dipX = 96.0 / Math.Max(1, (int)monitor.DpiX);
        _dipY = 96.0 / Math.Max(1, (int)monitor.DpiY);
        InitializeComponent();
        BuildChrome();
        Left = monitor.Bounds.X * _dipX;
        Top = monitor.Bounds.Y * _dipY;
        Width = Math.Max(1, monitor.Bounds.Width * _dipX);
        Height = Math.Max(1, monitor.Bounds.Height * _dipY);
        LayoutChrome();
        Loaded += (_, _) =>
        {
            _armedUtc = DateTime.UtcNow;
            LayoutChrome();
            Activate();
        };
    }

    public PixelBuffer? Committed { get; private set; }
    public bool PinRequested { get; private set; }
    public bool Accepted { get; private set; }
    public PixelRect Selection => _selection;
    public PixelBuffer Peek() => Bake();

    public event Action<PixelRect>? RegionChanged;
    public event Action? SaveClicked;
    public event Action<string>? StatusMessage;

    /// <summary>Drop full-screen capture frames, freeze/preview BitmapSources, and bake cache after annotate ends.</summary>
    public void ReleaseSessionBuffers()
    {
        foreach (MonitorCapture frame in _frames)
        {
            frame.Buffer.ReleasePixels();
        }

        _frames = Array.Empty<MonitorCapture>();
        _cachedBake?.ReleasePixels();
        _cachedBake = null;
        _original.ReleasePixels();
        _bakeDirty = true;
        _ops.Clear();
        _redo.Clear();
        _stroke = null;

        // Full-monitor freeze + live-viewport preview hold large BitmapSources — drop on close/cancel/commit.
        _freezeImage.Source = null;
        _freezeSource = null;
        _preview.Source = null;
        _previewBmp = null;
        _draftLayer.Children.Clear();
        _draft = null;
        _arrowHeadDraft = null;
        ReleaseMosaicDraft();
        _liveViewport = false;
    }

    private enum Handle
    {
        None, N, S, E, W, NE, NW, SE, SW,
    }

    private void BuildChrome()
    {
        BuildFreezeBackdrop();
        _preview.Stretch = Stretch.Fill;
        _preview.SnapsToDevicePixels = true;
        RenderOptions.SetBitmapScalingMode(_preview, BitmapScalingMode.NearestNeighbor);
        _imageHost.Background = Brushes.Transparent;
        _imageHost.Children.Add(_preview);
        _imageHost.Children.Add(_draftLayer);
        _imageHost.Children.Add(_inputLayer);
        _imageHost.MouseLeftButtonDown += OnCanvasDown;
        _imageHost.MouseMove += OnCanvasMove;
        _imageHost.MouseLeftButtonUp += OnCanvasUp;
        _imageHost.MouseDown += OnImageMouseDown;
        RefreshHostCursor();

        _textEntry.Visibility = Visibility.Collapsed;
        _textEntry.FontSize = 16;
        _textEntry.MinWidth = 120;
        _textEntry.CaretBrush = Brushes.White;
        _textEntry.Foreground = new SolidColorBrush(Color.FromRgb(0xDC, 0x28, 0x28));
        _textEntry.Background = new SolidColorBrush(Color.FromArgb(0xAA, 0, 0, 0));
        _textEntry.BorderBrush = new SolidColorBrush(Color.FromRgb(0xDC, 0x28, 0x28));
        _textEntry.AcceptsReturn = false;
        _textEntry.KeyDown += OnTextKey;
        _inputLayer.Children.Add(_textEntry);

        foreach (Rectangle dim in new[] { _dimTop, _dimBottom, _dimLeft, _dimRight })
        {
            dim.Fill = DimBrush;
            dim.IsHitTestVisible = true;
            dim.MouseLeftButtonDown += OnDimClick;
        }

        _border.Stroke = CommitInk;
        _border.StrokeThickness = AnnotateToolbar.StrokePx;
        _border.StrokeDashArray = null; // Joe: solid blue, no dashed frame
        _border.Fill = Brushes.Transparent;
        _border.IsHitTestVisible = false;

        for (int i = 0; i < _anchors.Length; i++)
        {
            var dot = new Ellipse
            {
                Width = AnnotateToolbar.AnchorDiameter,
                Height = AnnotateToolbar.AnchorDiameter,
                Fill = AnchorFill,
                Stroke = CommitInk,
                StrokeThickness = 1.5,
            };
            dot.MouseLeftButtonDown += OnAnchorDown;
            _anchors[i] = dot;
        }

        _sizeText.Foreground = Brushes.White;
        _sizeText.FontFamily = new FontFamily("Segoe UI, Consolas, Microsoft YaHei UI");
        _sizeText.FontSize = 11;
        _sizeChip.Background = ChipBg;
        _sizeChip.CornerRadius = new CornerRadius(3);
        _sizeChip.Padding = new Thickness(6, 2, 6, 2);
        _sizeChip.Child = _sizeText;
        _sizeChip.IsHitTestVisible = false;

        BuildToolbar();

        // z-order: freeze → dims (hole) → bake preview → chrome
        Root.Children.Add(_freezeImage);
        Root.Children.Add(_dimTop);
        Root.Children.Add(_dimBottom);
        Root.Children.Add(_dimLeft);
        Root.Children.Add(_dimRight);
        Root.Children.Add(_imageHost);
        Root.Children.Add(_border);
        Root.Children.Add(_sizeChip);
        Root.Children.Add(_toolbar);
        Root.Children.Add(_subBar);
        Root.Children.Add(_statusChip);
        foreach (Ellipse dot in _anchors)
        {
            Root.Children.Add(dot);
        }
    }

    private void BuildToolbar()
    {
        _toolbar.Background = ToolbarBg;
        _toolbar.BorderBrush = ToolbarLine;
        _toolbar.BorderThickness = new Thickness(1);
        _toolbar.CornerRadius = new CornerRadius(2);
        _toolbar.Height = AnnotateToolbar.Height;
        _toolbar.Padding = new Thickness(8, 2, 8, 2);
        var row = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
        foreach (AnnotateTool tool in AnnotateToolbar.Order)
        {
            if (tool is AnnotateTool.Undo or AnnotateTool.Close)
            {
                row.Children.Add(MakeSeparator());
            }

            ToolButton button = MakeButton(tool);
            _buttons[tool] = button;
            if (tool == AnnotateTool.Undo)
            {
                _undoButton = button;
            }

            if (tool == AnnotateTool.Redo)
            {
                _redoButton = button;
            }

            row.Children.Add(button);
        }

        _toolbar.Child = row;
        BuildSubBarShell();
        BuildStatusChip();
        RefreshHistory();
        RefreshSubBar();
    }

    private void BuildSubBarShell()
    {
        _subBar.Background = SubBarBg;
        _subBar.BorderBrush = Freeze(Color.FromRgb(0x1A, 0x1A, 0x1A));
        _subBar.BorderThickness = new Thickness(1);
        _subBar.CornerRadius = new CornerRadius(2);
        _subBar.Height = AnnotateToolbar.SubBarHeight;
        _subBar.Padding = new Thickness(8, 4, 8, 4);
        _subBar.Visibility = Visibility.Collapsed;
        _subBar.Child = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            VerticalAlignment = VerticalAlignment.Center,
        };
    }

    private void RefreshSubBar()
    {
        bool show = _tool is AnnotateTool t && AnnotateToolbar.ShowsPropertyBar(t);
        if (!show)
        {
            _subBar.Visibility = Visibility.Collapsed;
            _subBar.Child = null;
            return;
        }

        var row = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            VerticalAlignment = VerticalAlignment.Center,
        };

        switch (_tool)
        {
            case AnnotateTool.Curve:
                AddColorSwatches(row);
                AddSep(row);
                AddThicknessChips(row, AnnotateToolbar.ThicknessChips, _strokeThickness, v => _strokeThickness = v);
                AddSep(row);
                AddCurveStyleChips(row);
                if (_curveStyle == CurveStyle.Arrow)
                {
                    AddSep(row);
                    AddArrowDirChips(row);
                }
                break;
            case AnnotateTool.Shape:
                AddColorSwatches(row);
                AddSep(row);
                AddThicknessChips(row, AnnotateToolbar.ThicknessChips, _strokeThickness, v => _strokeThickness = v);
                AddSep(row);
                AddShapeKindChips(row);
                break;
            case AnnotateTool.Pencil:
                AddColorSwatches(row);
                AddSep(row);
                AddThicknessChips(row, AnnotateToolbar.ThicknessChips, _strokeThickness, v => _strokeThickness = v);
                break;
            case AnnotateTool.Marker:
                AddColorSwatches(row);
                AddSep(row);
                AddThicknessChips(row, AnnotateToolbar.MarkerThicknessChips, _markerThickness, v => _markerThickness = v);
                break;
            case AnnotateTool.Text:
                AddColorSwatches(row);
                AddSep(row);
                AddThicknessChips(row, AnnotateToolbar.TextSizeChips, _textSize, v =>
                {
                    _textSize = v;
                    _textEntry.FontSize = v;
                }, labelAsSize: true);
                SyncTextEntryInk();
                break;
            case AnnotateTool.Mosaic:
                AddThicknessChips(row, AnnotateToolbar.MosaicRadiusChips, _mosaicRadius, v => _mosaicRadius = v, tipPrefix: "笔刷");
                break;
            case AnnotateTool.Eraser:
                AddThicknessChips(row, AnnotateToolbar.EraserThicknessChips, _eraserThickness, v => _eraserThickness = v, tipPrefix: "橡皮");
                break;
        }

        _subBar.Child = row;
        _subBar.Visibility = Visibility.Visible;
        _subBar.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
        _cachedSubBarW = Math.Max(80, _subBar.DesiredSize.Width);
    }

    private void AddSep(StackPanel row)
    {
        row.Children.Add(new Border
        {
            Width = 8,
            Height = 20,
            Child = new Rectangle
            {
                Width = 1,
                Height = 14,
                Fill = SubBarSep,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
            },
        });
    }

    private void AddColorSwatches(StackPanel row)
    {
        foreach ((byte R, byte G, byte B) rgb in AnnotateToolbar.Palette)
        {
            byte b = rgb.B, g = rgb.G, r = rgb.R;
            bool on = _strokeB == b && _strokeG == g && _strokeR == r;
            var swatch = new Border
            {
                Width = 18,
                Height = 18,
                Margin = new Thickness(3, 0, 3, 0),
                CornerRadius = new CornerRadius(2),
                Background = Freeze(Color.FromRgb(r, g, b)),
                BorderBrush = on ? Freeze(Color.FromRgb(0x4F, 0xC3, 0xF7)) : Freeze(Color.FromRgb(0x88, 0x88, 0x88)),
                BorderThickness = new Thickness(on ? 2 : 1),
                Cursor = Cursors.Hand,
                ToolTip = $"#{r:X2}{g:X2}{b:X2}",
            };
            swatch.MouseLeftButtonDown += (_, e) =>
            {
                _strokeB = b;
                _strokeG = g;
                _strokeR = r;
                SyncTextEntryInk();
                RefreshSubBar();
                e.Handled = true;
            };
            row.Children.Add(swatch);
        }
    }

    private void AddThicknessChips(
        StackPanel row,
        int[] chips,
        int current,
        Action<int> set,
        string tipPrefix = "粗细",
        bool labelAsSize = false)
    {
        string[] labels = labelAsSize ? ["小", "中", "大"] : ["细", "中", "粗"];
        for (int i = 0; i < chips.Length; i++)
        {
            int value = chips[i];
            string label = i < labels.Length ? labels[i] : value.ToString();
            bool on = current == value;
            Button btn = MakeSubChip(label, on, $"{tipPrefix} {value}");
            btn.Click += (_, _) =>
            {
                set(value);
                RefreshSubBar();
            };
            row.Children.Add(btn);
        }
    }

    private void AddCurveStyleChips(StackPanel row)
    {
        void Add(string label, CurveStyle style)
        {
            bool on = _curveStyle == style;
            Button btn = MakeSubChip(label, on, label);
            btn.Click += (_, _) =>
            {
                _curveStyle = style;
                RefreshSubBar();
                LayoutChrome(rebakePreview: false);
            };
            row.Children.Add(btn);
        }

        Add("实线", CurveStyle.Solid);
        Add("虚线", CurveStyle.Dashed);
        Add("箭头", CurveStyle.Arrow);
    }

    private void AddArrowDirChips(StackPanel row)
    {
        Button endBtn = MakeSubChip("→末", _arrowAtEnd, "箭头在终点");
        endBtn.Click += (_, _) =>
        {
            _arrowAtEnd = true;
            RefreshSubBar();
        };
        Button startBtn = MakeSubChip("←始", !_arrowAtEnd, "箭头在起点");
        startBtn.Click += (_, _) =>
        {
            _arrowAtEnd = false;
            RefreshSubBar();
        };
        row.Children.Add(endBtn);
        row.Children.Add(startBtn);
    }

    private void AddShapeKindChips(StackPanel row)
    {
        Button rect = MakeSubChip("矩形", !_ellipse, "矩形");
        rect.Click += (_, _) =>
        {
            _ellipse = false;
            RefreshSubBar();
        };
        Button ell = MakeSubChip("椭圆", _ellipse, "椭圆");
        ell.Click += (_, _) =>
        {
            _ellipse = true;
            RefreshSubBar();
        };
        row.Children.Add(rect);
        row.Children.Add(ell);
    }

    private Button MakeSubChip(string text, bool on, string tip)
    {
        var btn = new Button
        {
            Content = text,
            FontSize = 11,
            FontFamily = new FontFamily("Microsoft YaHei UI, Segoe UI"),
            Foreground = SubBarInk,
            Background = on ? SubBarChipOn : SubBarChipBg,
            BorderBrush = on ? Freeze(Color.FromRgb(0x4F, 0xC3, 0xF7)) : SubBarSep,
            BorderThickness = new Thickness(on ? 1.5 : 1),
            Padding = new Thickness(8, 2, 8, 2),
            Margin = new Thickness(2, 0, 2, 0),
            MinWidth = 36,
            Height = 24,
            Focusable = false,
            Cursor = Cursors.Hand,
            ToolTip = tip,
        };
        btn.Template = BuildSubChipTemplate();
        return btn;
    }

    private static ControlTemplate BuildSubChipTemplate()
    {
        var template = new ControlTemplate(typeof(Button));
        var border = new FrameworkElementFactory(typeof(Border));
        border.Name = "Bd";
        border.SetValue(Border.CornerRadiusProperty, new CornerRadius(3));
        border.SetValue(Border.SnapsToDevicePixelsProperty, true);
        border.SetBinding(
            Border.BackgroundProperty,
            new System.Windows.Data.Binding(Button.BackgroundProperty.Name)
            {
                RelativeSource = new System.Windows.Data.RelativeSource(
                    System.Windows.Data.RelativeSourceMode.TemplatedParent),
            });
        border.SetBinding(
            Border.BorderBrushProperty,
            new System.Windows.Data.Binding(Button.BorderBrushProperty.Name)
            {
                RelativeSource = new System.Windows.Data.RelativeSource(
                    System.Windows.Data.RelativeSourceMode.TemplatedParent),
            });
        border.SetBinding(
            Border.BorderThicknessProperty,
            new System.Windows.Data.Binding(Button.BorderThicknessProperty.Name)
            {
                RelativeSource = new System.Windows.Data.RelativeSource(
                    System.Windows.Data.RelativeSourceMode.TemplatedParent),
            });
        border.SetBinding(
            Border.PaddingProperty,
            new System.Windows.Data.Binding(Button.PaddingProperty.Name)
            {
                RelativeSource = new System.Windows.Data.RelativeSource(
                    System.Windows.Data.RelativeSourceMode.TemplatedParent),
            });
        var presenter = new FrameworkElementFactory(typeof(ContentPresenter));
        presenter.SetValue(HorizontalAlignmentProperty, HorizontalAlignment.Center);
        presenter.SetValue(VerticalAlignmentProperty, VerticalAlignment.Center);
        border.AppendChild(presenter);
        template.VisualTree = border;
        return template;
    }

    private void SyncTextEntryInk()
    {
        var ink = new SolidColorBrush(Color.FromRgb(_strokeR, _strokeG, _strokeB));
        ink.Freeze();
        _textEntry.Foreground = ink;
        _textEntry.BorderBrush = ink;
        _textEntry.FontSize = _textSize;
    }

    private SolidColorBrush StrokeBrush(byte alpha = 255)
    {
        var brush = new SolidColorBrush(Color.FromArgb(alpha, _strokeR, _strokeG, _strokeB));
        brush.Freeze();
        return brush;
    }

    private void BuildStatusChip()
    {
        _statusText.Foreground = Brushes.White;
        _statusText.FontSize = 12;
        _statusText.Margin = new Thickness(8, 4, 8, 4);
        _statusText.TextWrapping = TextWrapping.Wrap;
        _statusChip.Background = ChipBg;
        _statusChip.CornerRadius = new CornerRadius(3);
        _statusChip.Child = _statusText;
        _statusChip.Visibility = Visibility.Collapsed;
        _statusChip.IsHitTestVisible = false;
        _statusChip.MaxWidth = 320;
    }

    private static FrameworkElement MakeSeparator()
    {
        return new Border
        {
            Width = AnnotateToolbar.SeparatorWidth,
            Height = AnnotateToolbar.Height,
            Child = new Rectangle
            {
                Width = 1,
                Height = 16,
                Fill = SeparatorInk,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center,
            },
        };
    }

    private ToolButton MakeButton(AnnotateTool tool)
    {
        var button = new ToolButton(tool);
        button.Click += (_, _) => OnToolClick(tool);
        if (tool == AnnotateTool.Shape)
        {
            var menu = new ContextMenu();
            var rect = new MenuItem { Header = "矩形" };
            rect.Click += (_, _) =>
            {
                _ellipse = false;
                SelectTool(AnnotateTool.Shape);
            };
            var ell = new MenuItem { Header = "椭圆" };
            ell.Click += (_, _) =>
            {
                _ellipse = true;
                SelectTool(AnnotateTool.Shape);
            };
            menu.Items.Add(rect);
            menu.Items.Add(ell);
            button.ContextMenu = menu;
        }
        else if (tool == AnnotateTool.Curve)
        {
            var menu = new ContextMenu();
            var solid = new MenuItem { Header = "实线" };
            solid.Click += (_, _) =>
            {
                _curveStyle = CurveStyle.Solid;
                SelectTool(AnnotateTool.Curve);
            };
            var dashed = new MenuItem { Header = "虚线" };
            dashed.Click += (_, _) =>
            {
                _curveStyle = CurveStyle.Dashed;
                SelectTool(AnnotateTool.Curve);
            };
            var arrow = new MenuItem { Header = "箭头" };
            arrow.Click += (_, _) =>
            {
                _curveStyle = CurveStyle.Arrow;
                SelectTool(AnnotateTool.Curve);
            };
            menu.Items.Add(solid);
            menu.Items.Add(dashed);
            menu.Items.Add(arrow);
            button.ContextMenu = menu;
        }

        return button;
    }

    private void OnToolClick(AnnotateTool tool)
    {
        switch (tool)
        {
            case AnnotateTool.Undo:
                Undo();
                return;
            case AnnotateTool.Redo:
                Redo();
                return;
            case AnnotateTool.Close:
                TryCancel();
                return;
            case AnnotateTool.Pin:
                Finish(pin: true);
                return;
            case AnnotateTool.Save:
                SaveClicked?.Invoke();
                return;
            case AnnotateTool.Copy:
                Finish(pin: false);
                return;
            case AnnotateTool.Ocr:
                _ = RecognizeOcrAsync();
                return;
            case AnnotateTool.Pick:
                SelectTool(tool);
                return;
            case AnnotateTool.Shape:
                _ellipse = false;
                SelectTool(tool);
                return;
            default:
                SelectTool(tool);
                return;
        }
    }

    private void SelectTool(AnnotateTool tool)
    {
        _tool = tool;
        foreach (ToolButton button in _buttons.Values)
        {
            button.SetSelected(AnnotateToolbar.ShowsSelectedDot(button.Tool) && button.Tool == tool);
        }

        RefreshHostCursor();
        RefreshSubBar();
        LayoutChrome(rebakePreview: false);
    }

    private void RefreshHostCursor() =>
        _imageHost.Cursor = _tool is null ? Cursors.SizeAll : Cursors.Cross;

    private void BuildFreezeBackdrop()
    {
        MonitorCapture? frame = null;
        foreach (MonitorCapture candidate in _frames)
        {
            if (candidate.Monitor.Handle == _monitor.Handle
                || candidate.Monitor.Bounds.Equals(_monitor.Bounds))
            {
                frame = candidate;
                break;
            }
        }

        frame ??= _frames.Count > 0 ? _frames[0] : null;
        if (frame is null)
        {
            return;
        }

        _freezeSource = frame.Buffer.ToBitmapSource();
        _freezeImage.Source = _freezeSource;
        _freezeImage.Stretch = Stretch.Fill;
        _freezeImage.SnapsToDevicePixels = true;
        _freezeImage.IsHitTestVisible = false;
        RenderOptions.SetBitmapScalingMode(_freezeImage, BitmapScalingMode.NearestNeighbor);
        Canvas.SetLeft(_freezeImage, 0);
        Canvas.SetTop(_freezeImage, 0);
    }

    /// <summary>
    /// Snipaste-like live viewport: hide bake sticker so the frozen full-screen image
    /// shows through the dim hole at the current selection. No Bake during drag.
    /// </summary>
    private void EnterLiveViewport()
    {
        if (_liveViewport)
        {
            return;
        }

        _liveViewport = true;
        // Hide bake sticker entirely so the frozen full-screen image shows through the dim hole
        // at the live selection — same as Snipaste/WeChat (content follows position, not a sticker).
        _imageHost.Opacity = 0;
        _imageHost.IsHitTestVisible = false;
        _preview.Visibility = Visibility.Collapsed;
        _draftLayer.Visibility = Visibility.Collapsed;
        _inputLayer.Visibility = Visibility.Collapsed;
    }

    private void ExitLiveViewport()
    {
        if (!_liveViewport)
        {
            return;
        }

        _liveViewport = false;
        _imageHost.Opacity = 1;
        _imageHost.IsHitTestVisible = true;
        _preview.Visibility = Visibility.Visible;
        _draftLayer.Visibility = Visibility.Visible;
        _inputLayer.Visibility = Visibility.Visible;
    }

    private void LayoutChrome(bool rebakePreview = true)
    {
        // Full layout path only — never call this every mousemove during _moving/_resizing.
        DetachLiveTransform();
        if (rebakePreview)
        {
            ExitLiveViewport();
        }

        double winW = ActualWidth > 1 ? ActualWidth : Width;
        double winH = ActualHeight > 1 ? ActualHeight : Height;
        _freezeImage.Width = Math.Max(1, winW);
        _freezeImage.Height = Math.Max(1, winH);

        double selL = (_selection.X - _monitor.Bounds.X) * _dipX;
        double selT = (_selection.Y - _monitor.Bounds.Y) * _dipY;
        double selW = Math.Max(1, _selection.Width * _dipX);
        double selH = Math.Max(1, _selection.Height * _dipY);
        double workL = (_monitor.WorkArea.X - _monitor.Bounds.X) * _dipX;
        double workT = (_monitor.WorkArea.Y - _monitor.Bounds.Y) * _dipY;
        double workR = (_monitor.WorkArea.Right - _monitor.Bounds.X) * _dipX;
        double workB = (_monitor.WorkArea.Bottom - _monitor.Bounds.Y) * _dipY;

        if (rebakePreview)
        {
            PresentBuffer(Bake());
        }

        _preview.Width = selW;
        _preview.Height = selH;
        _imageHost.Width = selW;
        _imageHost.Height = selH;
        _draftLayer.Width = selW;
        _draftLayer.Height = selH;
        Canvas.SetLeft(_imageHost, selL);
        Canvas.SetTop(_imageHost, selT);

        Canvas.SetLeft(_border, selL);
        Canvas.SetTop(_border, selT);
        _border.Width = selW;
        _border.Height = selH;

        _laidSelL = selL;
        _laidSelT = selT;
        _laidSelW = selW;
        _laidSelH = selH;

        UpdateDims(selL, selT, selW, selH);
        PlaceAnchors(selL, selT, selW, selH);

        _sizeText.Text = CaptureUx.SizeChip(_selection.Width, _selection.Height);
        MeasureChromeSizes(force: rebakePreview);
        PlaceToolbarAndChip(selL, selT, selW, selH, workL, workT, workR, workB);
    }

    private void MeasureChromeSizes(bool force)
    {
        if (force || !string.Equals(_cachedChipText, _sizeText.Text, StringComparison.Ordinal))
        {
            _sizeChip.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
            _cachedChipW = Math.Max(8, _sizeChip.DesiredSize.Width);
            _cachedChipH = Math.Max(8, _sizeChip.DesiredSize.Height);
            _cachedChipText = _sizeText.Text;
        }

        if (force)
        {
            _toolbar.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
            _cachedBarW = Math.Max(120, _toolbar.DesiredSize.Width);
            if (_subBar.Visibility == Visibility.Visible)
            {
                _subBar.Measure(new Size(double.PositiveInfinity, double.PositiveInfinity));
                _cachedSubBarW = Math.Max(80, _subBar.DesiredSize.Width);
            }
        }
    }

    private void PlaceToolbarAndChip(
        double selL, double selT, double selW, double selH,
        double workL, double workT, double workR, double workB)
    {
        double chipW = _cachedChipW;
        double chipH = _cachedChipH;
        bool showSub = _subBar.Visibility == Visibility.Visible;
        double barW = showSub ? Math.Max(_cachedBarW, _cachedSubBarW) : _cachedBarW;
        double mainH = AnnotateToolbar.Height;
        double subH = AnnotateToolbar.SubBarHeight;

        AnnotateToolbar.StackPlacement place = AnnotateToolbar.PlaceStack(
            selL, selT, selW, selH,
            barW, mainH, subH, showSub,
            workL, workT, workR, workB);
        Canvas.SetLeft(_toolbar, place.Left);
        Canvas.SetTop(_toolbar, place.MainTop);
        if (showSub)
        {
            Canvas.SetLeft(_subBar, place.Left);
            Canvas.SetTop(_subBar, place.SubTop);
        }

        PlaceStatusChip();

        double chipX = selL + ((selW - chipW) / 2);
        double chipY = selT - chipH - 2;
        if (place.Above)
        {
            chipY = selT + 4;
        }

        if (chipY < 0)
        {
            chipY = selT + 4;
        }

        Canvas.SetLeft(_sizeChip, chipX);
        Canvas.SetTop(_sizeChip, chipY);
    }

    private void PlaceAnchors(double selL, double selT, double selW, double selH)
    {
        PlaceAnchor(0, selL, selT); // NW
        PlaceAnchor(1, selL + (selW / 2), selT); // N
        PlaceAnchor(2, selL + selW, selT); // NE
        PlaceAnchor(3, selL + selW, selT + (selH / 2)); // E
        PlaceAnchor(4, selL + selW, selT + selH); // SE
        PlaceAnchor(5, selL + (selW / 2), selT + selH); // S
        PlaceAnchor(6, selL, selT + selH); // SW
        PlaceAnchor(7, selL, selT + (selH / 2)); // W
        _anchors[0].Tag = Handle.NW;
        _anchors[1].Tag = Handle.N;
        _anchors[2].Tag = Handle.NE;
        _anchors[3].Tag = Handle.E;
        _anchors[4].Tag = Handle.SE;
        _anchors[5].Tag = Handle.S;
        _anchors[6].Tag = Handle.SW;
        _anchors[7].Tag = Handle.W;
    }

    private void UpdateDims(double selL, double selT, double selW, double selH)
    {
        double winW = ActualWidth > 1 ? ActualWidth : Width;
        double winH = ActualHeight > 1 ? ActualHeight : Height;
        SetDim(_dimTop, 0, 0, winW, Math.Max(0, selT));
        SetDim(_dimBottom, 0, selT + selH, winW, Math.Max(0, winH - selT - selH));
        SetDim(_dimLeft, 0, selT, Math.Max(0, selL), selH);
        SetDim(_dimRight, selL + selW, selT, Math.Max(0, winW - selL - selW), selH);
    }

    private void AttachLiveTransform()
    {
        if (_liveTxAttached)
        {
            return;
        }

        _liveTx.X = 0;
        _liveTx.Y = 0;
        // Do not transform _imageHost: bake sticker is hidden; freeze under dim hole is the viewport.
        _border.RenderTransform = _liveTx;
        _sizeChip.RenderTransform = _liveTx;
        _toolbar.RenderTransform = _liveTx;
        _subBar.RenderTransform = _liveTx;
        foreach (Ellipse dot in _anchors)
        {
            dot.RenderTransform = _liveTx;
        }

        _liveTxAttached = true;
    }

    private void DetachLiveTransform()
    {
        if (!_liveTxAttached)
        {
            _liveTx.X = 0;
            _liveTx.Y = 0;
            return;
        }

        _liveTx.X = 0;
        _liveTx.Y = 0;
        _imageHost.RenderTransform = null;
        _border.RenderTransform = null;
        _sizeChip.RenderTransform = null;
        _toolbar.RenderTransform = null;
        _subBar.RenderTransform = null;
        foreach (Ellipse dot in _anchors)
        {
            dot.RenderTransform = null;
        }

        _liveTxAttached = false;
    }

    private void HookLiveRendering()
    {
        if (_renderHooked)
        {
            return;
        }

        CompositionTarget.Rendering += OnLiveRendering;
        _renderHooked = true;
    }

    private void UnhookLiveRendering()
    {
        if (!_renderHooked)
        {
            return;
        }

        CompositionTarget.Rendering -= OnLiveRendering;
        _renderHooked = false;
        _dimsDirty = false;
    }

    private void OnLiveRendering(object? sender, EventArgs e)
    {
        if (_moving && _dimsDirty)
        {
            _dimsDirty = false;
            // Dims follow selection hole; transform already moved chrome — use live selection DIP.
            double selL = (_selection.X - _monitor.Bounds.X) * _dipX;
            double selT = (_selection.Y - _monitor.Bounds.Y) * _dipY;
            UpdateDims(selL, selT, _laidSelW, _laidSelH);
        }

        if (!_moving && !_resizing)
        {
            UnhookLiveRendering();
        }
    }

    /// <summary>
    /// Move path: translate chrome via RenderTransform; hide bake sticker so freeze shows
    /// through the dim hole at the new position (Snipaste real-time background).
    /// </summary>
    private void ApplyLiveMove()
    {
        EnterLiveViewport();
        AttachLiveTransform();
        _liveTx.X = (_selection.X - _moveOrigin.X) * _dipX;
        _liveTx.Y = (_selection.Y - _moveOrigin.Y) * _dipY;
        _dimsDirty = true;
        HookLiveRendering();
    }

    /// <summary>
    /// Resize path: rubber-band chrome + dim hole. Bake sticker hidden; freeze viewport
    /// shows through the hole. Optional CroppedBitmap kept in sync for host size.
    /// </summary>
    private void ApplyRubberBand(PixelRect sel)
    {
        EnterLiveViewport();
        double selL = (sel.X - _monitor.Bounds.X) * _dipX;
        double selT = (sel.Y - _monitor.Bounds.Y) * _dipY;
        double selW = Math.Max(1, sel.Width * _dipX);
        double selH = Math.Max(1, sel.Height * _dipY);
        double workL = (_monitor.WorkArea.X - _monitor.Bounds.X) * _dipX;
        double workT = (_monitor.WorkArea.Y - _monitor.Bounds.Y) * _dipY;
        double workR = (_monitor.WorkArea.Right - _monitor.Bounds.X) * _dipX;
        double workB = (_monitor.WorkArea.Bottom - _monitor.Bounds.Y) * _dipY;

        Canvas.SetLeft(_border, selL);
        Canvas.SetTop(_border, selT);
        _border.Width = selW;
        _border.Height = selH;

        PlaceAnchors(selL, selT, selW, selH);
        UpdateDims(selL, selT, selW, selH);

        _sizeText.Text = CaptureUx.SizeChip(sel.Width, sel.Height);
        // No Measure during drag — estimate chip width from glyph count if text changed.
        if (!string.Equals(_cachedChipText, _sizeText.Text, StringComparison.Ordinal))
        {
            _cachedChipW = Math.Max(48, 12 + (_sizeText.Text.Length * 7));
            _cachedChipText = _sizeText.Text;
        }

        PlaceToolbarAndChip(selL, selT, selW, selH, workL, workT, workR, workB);
    }

    private void BeginRubberResize()
    {
        DetachLiveTransform();
        EnterLiveViewport();
        _rubberSelection = _selection;
    }

    private static void SetDim(Rectangle rect, double x, double y, double w, double h)
    {
        Canvas.SetLeft(rect, x);
        Canvas.SetTop(rect, y);
        rect.Width = Math.Max(0, w);
        rect.Height = Math.Max(0, h);
    }

    private void OnDimClick(object sender, MouseButtonEventArgs e)
    {
        if (TryCancel())
        {
            e.Handled = true;
        }
    }

    private bool IsArmed =>
        (DateTime.UtcNow - _armedUtc).TotalMilliseconds >= CaptureUx.AnnotateArmMs;

    private bool TryCancel()
    {
        if (!IsArmed)
        {
            return false;
        }

        Close();
        return true;
    }

    private void PlaceAnchor(int index, double x, double y)
    {
        double d = AnnotateToolbar.AnchorDiameter;
        Canvas.SetLeft(_anchors[index], x - (d / 2));
        Canvas.SetTop(_anchors[index], y - (d / 2));
    }

    private void OnPreviewMouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Middle)
        {
            Finish(pin: true);
            e.Handled = true;
        }
    }

    private void OnRightUp(object sender, MouseButtonEventArgs e)
    {
        if (_textEntry.Visibility == Visibility.Visible)
        {
            return;
        }

        if (e.OriginalSource is MenuItem || e.OriginalSource is ContextMenu
            || _buttons.Values.Any(b => b.ContextMenu is { IsOpen: true })
            || IsUnder(_toolbar, e.OriginalSource as DependencyObject))
        {
            return;
        }

        if (TryCancel())
        {
            e.Handled = true;
        }
    }

    private static bool IsUnder(DependencyObject root, DependencyObject? node)
    {
        while (node is not null)
        {
            if (ReferenceEquals(node, root))
            {
                return true;
            }

            node = VisualTreeHelper.GetParent(node);
        }

        return false;
    }

    private void ArmKeyboardFocus()
    {
        try
        {
            Activate();
            Focus();
            Keyboard.Focus(this);
        }
        catch
        {
        }
    }

    private void OnPreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (_textEntry.Visibility == Visibility.Visible)
        {
            return;
        }

        Key key = e.Key == Key.System ? e.SystemKey : e.Key;
        bool ctrl = (Keyboard.Modifiers & ModifierKeys.Control) == ModifierKeys.Control
            || Keyboard.IsKeyDown(Key.LeftCtrl)
            || Keyboard.IsKeyDown(Key.RightCtrl);

        if (key == Key.Escape)
        {
            Close();
            e.Handled = true;
            return;
        }

        // Enter / Ctrl+C = same as Copy toolbar button (clipboard + exit, no pin).
        if (key == Key.Enter || (key == Key.C && ctrl))
        {
            Finish(pin: false);
            e.Handled = true;
            return;
        }

        if (key == Key.Z && ctrl)
        {
            Undo();
            e.Handled = true;
            return;
        }

        if (key == Key.Y && ctrl)
        {
            Redo();
            e.Handled = true;
        }
    }

    private void OnImageMouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ClickCount >= 2 && e.ChangedButton == MouseButton.Left)
        {
            Finish(pin: false);
            e.Handled = true;
        }
    }

    private void OnAnchorDown(object sender, MouseButtonEventArgs e)
    {
        if (sender is not Ellipse ellipse || ellipse.Tag is not Handle handle)
        {
            return;
        }

        _resizing = true;
        _handle = handle;
        _resizeOrigin = _selection;
        BeginRubberResize();
        CaptureMouse();
        e.Handled = true;
    }

    private void OnCanvasDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ClickCount >= 2)
        {
            return;
        }

        Point pos = e.GetPosition(_preview);
        if (_tool == AnnotateTool.Text)
        {
            ShowTextBox(pos);
            e.Handled = true;
            return;
        }

        if (_tool == AnnotateTool.Pick)
        {
            PickColorAt(pos);
            e.Handled = true;
            return;
        }

        if (_tool is null)
        {
            // 无绘制工具：在预览上拖动 = 平移选区（保持 W×H）
            _moving = true;
            _moveOrigin = _selection;
            _moveStartVirtual = ToVirtual(e.GetPosition(this));
            EnterLiveViewport();
            AttachLiveTransform();
            HookLiveRendering();
            CaptureMouse();
            e.Handled = true;
            return;
        }

        _dragging = true;
        _startDip = pos;
        int px = ToPxX(pos.X);
        int py = ToPxY(pos.Y);
        _stroke = [(px, py)];
        _imageHost.CaptureMouse();
        _draftLayer.Children.Clear();
        _arrowHeadDraft = null;
        ReleaseMosaicDraft();

        if (_tool == AnnotateTool.Mosaic)
        {
            _mosaicDraft = Bake().Clone();
            _mosaicDraftLast = 0;
            PixelDraw.MosaicStamp(_mosaicDraft, px, py, _mosaicRadius);
            PresentBuffer(_mosaicDraft);
            _draft = null;
            e.Handled = true;
            return;
        }

        if (_tool is AnnotateTool.Pencil or AnnotateTool.Marker or AnnotateTool.Eraser or AnnotateTool.Curve)
        {
            double thick = _tool switch
            {
                AnnotateTool.Marker => _markerThickness,
                AnnotateTool.Eraser => _eraserThickness,
                AnnotateTool.Curve => _strokeThickness,
                _ => Math.Max(1, _strokeThickness - 1),
            };
            Brush stroke = _tool == AnnotateTool.Eraser
                ? Freeze(Color.FromArgb(0x90, 0xFF, 0xFF, 0xFF))
                : _tool == AnnotateTool.Marker
                    ? StrokeBrush(0x70)
                    : StrokeBrush();
            var line = new Polyline
            {
                Stroke = stroke,
                StrokeThickness = thick,
                StrokeLineJoin = PenLineJoin.Round,
                StrokeStartLineCap = PenLineCap.Round,
                StrokeEndLineCap = PenLineCap.Round,
            };
            if (_tool == AnnotateTool.Curve && _curveStyle == CurveStyle.Dashed)
            {
                line.StrokeDashArray = new DoubleCollection { 4, 3 };
            }

            line.Points.Add(pos);
            _draft = line;
            if (_tool == AnnotateTool.Curve && _curveStyle == CurveStyle.Arrow)
            {
                _arrowHeadDraft = new Polyline
                {
                    Stroke = StrokeBrush(),
                    StrokeThickness = _strokeThickness,
                    StrokeLineJoin = PenLineJoin.Round,
                    StrokeStartLineCap = PenLineCap.Round,
                    StrokeEndLineCap = PenLineCap.Round,
                };
                _draftLayer.Children.Add(_arrowHeadDraft);
            }
        }
        else
        {
            // Shape rubber-band
            _draft = new System.Windows.Shapes.Rectangle
            {
                Stroke = StrokeBrush(),
                StrokeThickness = Math.Max(1, _strokeThickness * _dipX),
                Fill = Brushes.Transparent,
            };
            if (_ellipse)
            {
                _draft = new System.Windows.Shapes.Ellipse
                {
                    Stroke = StrokeBrush(),
                    StrokeThickness = Math.Max(1, _strokeThickness * _dipX),
                    Fill = Brushes.Transparent,
                };
            }

            Canvas.SetLeft(_draft, pos.X);
            Canvas.SetTop(_draft, pos.Y);
        }

        if (_draft is not null)
        {
            _draftLayer.Children.Add(_draft);
        }

        e.Handled = true;
    }

    private void OnCanvasMove(object sender, MouseEventArgs e)
    {
        if (_resizing)
        {
            (int x, int y) = ToVirtual(e.GetPosition(this));
            PixelRect next = ResizeRect(_resizeOrigin, _handle, x, y);
            if (next.Width >= CaptureUx.MinCommitPx && next.Height >= CaptureUx.MinCommitPx)
            {
                _selection = next;
                _rubberSelection = next;
                // Border/anchors only — keep 1:1 with cursor; Image deferred to CommitResize.
                ApplyRubberBand(next);
            }

            return;
        }

        if (_moving)
        {
            (int x, int y) = ToVirtual(e.GetPosition(this));
            int dx = x - _moveStartVirtual.X;
            int dy = y - _moveStartVirtual.Y;
            _selection = CaptureUx.MoveRect(_moveOrigin, dx, dy, ClampBounds());
            // Transform only — Image Width/Height/Source untouched until CommitMove.
            ApplyLiveMove();
            return;
        }

        if (!_dragging)
        {
            return;
        }

        Point pos = e.GetPosition(_preview);
        int px = ToPxX(pos.X);
        int py = ToPxY(pos.Y);
        _stroke?.Add((px, py));

        if (_tool == AnnotateTool.Mosaic && _mosaicDraft is not null && _stroke is not null)
        {
            int from = Math.Max(0, _mosaicDraftLast);
            if (from < _stroke.Count - 1)
            {
                var segment = _stroke.GetRange(from, _stroke.Count - from);
                PixelDraw.MosaicBrush(_mosaicDraft, segment, _mosaicRadius);
                _mosaicDraftLast = _stroke.Count - 1;
                PresentBuffer(_mosaicDraft);
            }

            return;
        }

        if (_draft is null)
        {
            return;
        }

        if (_draft is Polyline poly)
        {
            poly.Points.Add(pos);
            if (_tool == AnnotateTool.Curve && poly.Points.Count > 1)
            {
                poly.Points[1] = pos;
                while (poly.Points.Count > 2)
                {
                    poly.Points.RemoveAt(2);
                }

                if (_curveStyle == CurveStyle.Arrow && _arrowHeadDraft is not null)
                {
                    UpdateArrowHeadDraft(_startDip, pos);
                }
            }

            return;
        }

        double x0 = Math.Min(_startDip.X, pos.X);
        double y0 = Math.Min(_startDip.Y, pos.Y);
        _draft.Width = Math.Abs(pos.X - _startDip.X);
        _draft.Height = Math.Abs(pos.Y - _startDip.Y);
        Canvas.SetLeft(_draft, x0);
        Canvas.SetTop(_draft, y0);
    }

    protected override void OnMouseMove(MouseEventArgs e)
    {
        base.OnMouseMove(e);
        if (_resizing || _moving)
        {
            OnCanvasMove(this, e);
        }
    }

    protected override void OnMouseLeftButtonUp(MouseButtonEventArgs e)
    {
        base.OnMouseLeftButtonUp(e);
        if (_resizing)
        {
            _resizing = false;
            _handle = Handle.None;
            UnhookLiveRendering();
            if (IsMouseCaptured)
            {
                ReleaseMouseCapture();
            }

            CommitResize();
            e.Handled = true;
            return;
        }

        if (_moving)
        {
            _moving = false;
            UnhookLiveRendering();
            if (IsMouseCaptured)
            {
                ReleaseMouseCapture();
            }

            CommitMove();
            e.Handled = true;
        }
    }

    private void OnCanvasUp(object sender, MouseButtonEventArgs e)
    {
        if (_resizing || _moving)
        {
            return;
        }

        if (!_dragging)
        {
            return;
        }

        _dragging = false;
        if (_imageHost.IsMouseCaptured)
        {
            _imageHost.ReleaseMouseCapture();
        }

        Point pos = e.GetPosition(_preview);
        _draftLayer.Children.Clear();
        _draft = null;
        _arrowHeadDraft = null;
        IReadOnlyList<(int X, int Y)> stroke = _stroke ?? [];
        _stroke = null;

        if (_tool == AnnotateTool.Mosaic)
        {
            ReleaseMosaicDraft();
            if (stroke.Count == 0)
            {
                RefreshPreview();
                return;
            }

            Push(new MosaicStrokeOp(stroke, _mosaicRadius));
            return;
        }

        if (_tool is AnnotateTool.Pencil or AnnotateTool.Marker or AnnotateTool.Eraser)
        {
            if (stroke.Count == 0)
            {
                return;
            }

            if (_tool == AnnotateTool.Eraser)
            {
                Push(new EraseOp(stroke, _eraserThickness));
            }
            else if (_tool == AnnotateTool.Marker)
            {
                Push(new StrokeOp(stroke, _markerThickness, _strokeB, _strokeG, _strokeR, 110));
            }
            else
            {
                Push(new StrokeOp(stroke, Math.Max(1, _strokeThickness - 1), _strokeB, _strokeG, _strokeR, 255));
            }

            return;
        }

        if (_tool == AnnotateTool.Curve)
        {
            int x0 = ToPxX(_startDip.X);
            int y0 = ToPxY(_startDip.Y);
            int x1 = ToPxX(pos.X);
            int y1 = ToPxY(pos.Y);
            int thick = Math.Max(1, _strokeThickness);
            if (_curveStyle == CurveStyle.Arrow)
            {
                if (_arrowAtEnd)
                {
                    Push(new ArrowOp(x0, y0, x1, y1, thick, _strokeB, _strokeG, _strokeR));
                }
                else
                {
                    Push(new ArrowOp(x1, y1, x0, y0, thick, _strokeB, _strokeG, _strokeR));
                }
            }
            else if (_curveStyle == CurveStyle.Dashed)
            {
                Push(new StrokeOp(
                    [(x0, y0), (x1, y1)],
                    thick,
                    _strokeB,
                    _strokeG,
                    _strokeR,
                    255,
                    dashed: true));
            }
            else
            {
                Push(new StrokeOp(
                    [(x0, y0), (x1, y1)],
                    thick,
                    _strokeB,
                    _strokeG,
                    _strokeR,
                    255));
            }

            return;
        }

        PixelRect rect = PixelRect.FromCorners(ToPxX(_startDip.X), ToPxY(_startDip.Y), ToPxX(pos.X), ToPxY(pos.Y));
        if (rect.Width < 2 && rect.Height < 2)
        {
            return;
        }

        int shapeThick = Math.Max(1, _strokeThickness);
        if (_tool == AnnotateTool.Shape && _ellipse)
        {
            Push(new EllipseOp(rect, shapeThick, _strokeB, _strokeG, _strokeR));
            return;
        }

        if (_tool == AnnotateTool.Shape)
        {
            Push(new RectOp(rect, shapeThick, _strokeB, _strokeG, _strokeR));
        }
    }

    private void CommitResize()
    {
        if (_selection.Width < CaptureUx.MinCommitPx || _selection.Height < CaptureUx.MinCommitPx)
        {
            return;
        }

        PixelBuffer nextCrop = RegionCropper.Crop(_frames, _selection);
        _original.ReleasePixels();
        _original = nextCrop;
        _cachedBake?.ReleasePixels();
        _cachedBake = null;
        ReleaseMosaicDraft();
        _ops.Clear();
        _redo.Clear();
        _bakeDirty = true;
        // Selection size may change — drop WriteableBitmap so PresentBuffer reallocates.
        _previewBmp = null;
        RefreshHistory();
        LayoutChrome();
        RegionChanged?.Invoke(_selection);
    }

    private void CommitMove()
    {
        if (_selection.Width < CaptureUx.MinCommitPx || _selection.Height < CaptureUx.MinCommitPx)
        {
            return;
        }

        // v1：平移后重裁，清空标注栈（坐标相对旧裁剪区）
        PixelBuffer nextCrop = RegionCropper.Crop(_frames, _selection);
        _original.ReleasePixels();
        _original = nextCrop;
        _cachedBake?.ReleasePixels();
        _cachedBake = null;
        ReleaseMosaicDraft();
        _ops.Clear();
        _redo.Clear();
        _bakeDirty = true;
        RefreshHistory();
        LayoutChrome();
        RegionChanged?.Invoke(_selection);
    }

    private PixelRect ClampBounds()
    {
        var list = new List<PixelRect>(_frames.Count);
        foreach (MonitorCapture frame in _frames)
        {
            list.Add(frame.Monitor.Bounds);
        }

        return CaptureUx.UnionMonitorBounds(list, _monitor.Bounds);
    }

    private static PixelRect ResizeRect(PixelRect start, Handle handle, int x, int y)
    {
        int l = start.X;
        int t = start.Y;
        int r = start.Right;
        int b = start.Bottom;
        switch (handle)
        {
            case Handle.N:
                t = y;
                break;
            case Handle.S:
                b = y;
                break;
            case Handle.E:
                r = x;
                break;
            case Handle.W:
                l = x;
                break;
            case Handle.NE:
                return ScaleCorner(start, start.X, start.Bottom, x, y);
            case Handle.NW:
                return ScaleCorner(start, start.Right, start.Bottom, x, y);
            case Handle.SE:
                return ScaleCorner(start, start.X, start.Y, x, y);
            case Handle.SW:
                return ScaleCorner(start, start.Right, start.Y, x, y);
        }

        return PixelRect.FromCorners(l, t, r, b);
    }

    private static PixelRect ScaleCorner(PixelRect start, int fixedX, int fixedY, int x, int y)
    {
        if (start.Width <= 0 || start.Height <= 0)
        {
            return PixelRect.FromCorners(fixedX, fixedY, x, y);
        }

        double sx = (x - fixedX) / (double)start.Width;
        double sy = (y - fixedY) / (double)start.Height;
        double scale = Math.Abs(sx) > Math.Abs(sy) ? sx : sy;
        if (Math.Abs(scale) < 0.01)
        {
            scale = 0.01;
        }

        int w = Math.Max(CaptureUx.MinCommitPx, (int)Math.Round(start.Width * Math.Abs(scale)));
        int h = Math.Max(CaptureUx.MinCommitPx, (int)Math.Round(start.Height * Math.Abs(scale)));
        int nx = x < fixedX ? fixedX - w : fixedX;
        int ny = y < fixedY ? fixedY - h : fixedY;
        return new PixelRect(nx, ny, w, h);
    }

    private void ShowTextBox(Point dip)
    {
        Canvas.SetLeft(_textEntry, dip.X);
        Canvas.SetTop(_textEntry, dip.Y);
        _textEntry.Tag = dip;
        _textEntry.Text = "";
        _textEntry.Visibility = Visibility.Visible;
        _textEntry.Focus();
    }

    private void OnTextKey(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            _textEntry.Visibility = Visibility.Collapsed;
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Enter)
        {
            return;
        }

        CommitText();
        e.Handled = true;
    }

    private void CommitText()
    {
        if (_textEntry.Visibility != Visibility.Visible)
        {
            return;
        }

        string text = _textEntry.Text.Trim();
        _textEntry.Visibility = Visibility.Collapsed;
        if (text.Length > 0 && _textEntry.Tag is Point dip)
        {
            Push(new TextOp(ToPxX(dip.X), ToPxY(dip.Y), text, _strokeB, _strokeG, _strokeR, _textSize));
        }
    }

    private void Push(IAnnotationOp op)
    {
        _ops.Add(op);
        _redo.Clear();
        _bakeDirty = true;
        RefreshPreview();
        RefreshHistory();
    }

    private void Undo()
    {
        if (_ops.Count == 0)
        {
            return;
        }

        IAnnotationOp op = _ops[^1];
        _ops.RemoveAt(_ops.Count - 1);
        _redo.Add(op);
        _bakeDirty = true;
        RefreshPreview();
        RefreshHistory();
    }

    private void Redo()
    {
        if (_redo.Count == 0)
        {
            return;
        }

        IAnnotationOp op = _redo[^1];
        _redo.RemoveAt(_redo.Count - 1);
        _ops.Add(op);
        _bakeDirty = true;
        RefreshPreview();
        RefreshHistory();
    }

    private void RefreshHistory()
    {
        SetHistory(_undoButton, _ops.Count > 0);
        SetHistory(_redoButton, _redo.Count > 0);
    }

    private static void SetHistory(ToolButton? button, bool enabled)
    {
        if (button is null)
        {
            return;
        }

        button.IsEnabled = enabled;
        button.Opacity = enabled ? 1 : 0.4;
    }

    private void RefreshPreview() => PresentBuffer(Bake());

    /// <summary>
    /// Push BGRA into a single reusable WriteableBitmap. Avoids BitmapSource.Create
    /// (full pixel copy + freeze) on every mosaic stamp / undo / tool commit.
    /// </summary>
    private void PresentBuffer(PixelBuffer buffer)
    {
        if (buffer.Width <= 0 || buffer.Height <= 0 || buffer.Bgra.Length == 0)
        {
            _preview.Source = null;
            _previewBmp = null;
            return;
        }

        if (_previewBmp is null
            || _previewBmp.PixelWidth != buffer.Width
            || _previewBmp.PixelHeight != buffer.Height)
        {
            _previewBmp = new WriteableBitmap(
                buffer.Width,
                buffer.Height,
                96,
                96,
                PixelFormats.Bgra32,
                null);
            _preview.Source = _previewBmp;
        }

        _previewBmp.WritePixels(
            new Int32Rect(0, 0, buffer.Width, buffer.Height),
            buffer.Bgra,
            buffer.Stride,
            0);
    }

    private void Finish(bool pin)
    {
        CommitText();
        Committed = Bake().Clone();
        PinRequested = pin;
        Accepted = true;
        Close();
    }

    private PixelBuffer Bake()
    {
        if (!_bakeDirty && _cachedBake is not null && _cachedBake.Width > 0)
        {
            return _cachedBake;
        }

        _cachedBake?.ReleasePixels();
        PixelBuffer result = _original.Clone();
        foreach (IAnnotationOp op in _ops)
        {
            op.Apply(result, _original);
        }

        _cachedBake = result;
        _bakeDirty = false;
        return result;
    }

    private async Task RecognizeOcrAsync()
    {
        if (_ocrBusy)
        {
            return;
        }

        _ocrBusy = true;
        ToolButton? ocrBtn = _buttons.GetValueOrDefault(AnnotateTool.Ocr);
        if (ocrBtn is not null)
        {
            ocrBtn.IsEnabled = false;
            ocrBtn.Opacity = 0.4;
        }

        try
        {
            PixelBuffer image = Bake().Clone();
            try
            {
                OcrResult result = await _ocr.RecognizeAsync(image).ConfigureAwait(true);
                if (result.Succeeded)
                {
                    if (TryCopyTextToClipboard(result.Text ?? ""))
                    {
                        NotifyStatus(WindowsOcrService.Copied);
                    }
                    else
                    {
                        NotifyStatus(WindowsOcrService.ClipboardFailed, hardFailure: false);
                    }
                }
                else if (!string.IsNullOrEmpty(result.Error))
                {
                    bool hard = result.Error == WindowsOcrService.NoLanguage
                        || result.Error == NullOcrService.NotEnabled;
                    NotifyStatus(result.Error!, hardFailure: hard);
                }
            }
            finally
            {
                image.ReleasePixels();
            }
        }
        catch (OperationCanceledException)
        {
            // 静默：取消本次识别，栏仍在
        }
        catch
        {
            NotifyStatus(WindowsOcrService.Failed);
        }
        finally
        {
            _ocrBusy = false;
            if (ocrBtn is not null)
            {
                ocrBtn.IsEnabled = true;
                ocrBtn.Opacity = 1;
            }
        }
    }

    private bool TryCopyTextToClipboard(string text)
    {
        try
        {
            void Set()
            {
                var data = new System.Windows.DataObject();
                data.SetData(System.Windows.DataFormats.UnicodeText, text);
                System.Windows.Clipboard.SetDataObject(data, true);
            }

            if (Dispatcher.CheckAccess())
            {
                Set();
            }
            else
            {
                Dispatcher.Invoke(Set);
            }

            return true;
        }
        catch
        {
            return false;
        }
    }

    private void PickColorAt(Point dip)
    {
        int x = ToPxX(dip.X);
        int y = ToPxY(dip.Y);
        PixelBuffer bake = Bake();
        if (!bake.TryGetPixel(x, y, out byte b, out byte g, out byte r, out _))
        {
            NotifyStatus(ColorPick.Fail);
            return;
        }

        string hex = ColorPick.ToHex(r, g, b);
        if (ColorPick.TryCopyHex(hex, out string? error))
        {
            NotifyStatus(string.Format(ColorPick.CopiedFmt, hex));
        }
        else
        {
            NotifyStatus(error ?? ColorPick.Fail);
        }
    }

    private void NotifyStatus(string message, bool hardFailure = false)
    {
        if (string.IsNullOrEmpty(message))
        {
            return;
        }

        ShowToolbarStatus(message);

        if (_reportStatus is not null)
        {
            _reportStatus(message);
        }
        else
        {
            StatusMessage?.Invoke(message);
        }

        // Hard failures only: language pack / stub — prefer toast; MessageBox as last resort when no tray path.
        if (hardFailure && _reportStatus is null)
        {
            try
            {
                System.Windows.MessageBox.Show(this, message, "Suite", MessageBoxButton.OK, MessageBoxImage.Information);
            }
            catch
            {
            }
        }
    }

    private void ShowToolbarStatus(string message)
    {
        _statusText.Text = message;
        _statusChip.Visibility = Visibility.Visible;
        PlaceStatusChip();
        _statusHideCts?.Cancel();
        _statusHideCts?.Dispose();
        var cts = new CancellationTokenSource();
        _statusHideCts = cts;
        _ = HideStatusAfterAsync(cts.Token);
    }

    private async Task HideStatusAfterAsync(CancellationToken token)
    {
        try
        {
            await Task.Delay(2200, token).ConfigureAwait(true);
            if (!token.IsCancellationRequested)
            {
                _statusChip.Visibility = Visibility.Collapsed;
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private void PlaceStatusChip()
    {
        if (_statusChip.Visibility != Visibility.Visible)
        {
            return;
        }

        _statusChip.Measure(new Size(320, 80));
        double w = Math.Max(48, _statusChip.DesiredSize.Width);
        double h = Math.Max(22, _statusChip.DesiredSize.Height);
        double barLeft = Canvas.GetLeft(_toolbar);
        double barTop = Canvas.GetTop(_toolbar);
        if (double.IsNaN(barLeft) || double.IsNaN(barTop))
        {
            barLeft = 8;
            barTop = 8;
        }

        // Sit just above the toolbar stack when there is room; otherwise below.
        double stackH = AnnotateToolbar.Height;
        if (_subBar.Visibility == Visibility.Visible)
        {
            stackH += AnnotateToolbar.SubBarGap + AnnotateToolbar.SubBarHeight;
            double subTop = Canvas.GetTop(_subBar);
            if (!double.IsNaN(subTop))
            {
                barTop = Math.Min(barTop, subTop);
            }
        }

        double top = barTop - h - 6;
        if (top < 4)
        {
            top = barTop + stackH + 6;
        }

        Canvas.SetLeft(_statusChip, barLeft);
        Canvas.SetTop(_statusChip, top);
        _statusChip.Width = w;
    }

    private void UpdateArrowHeadDraft(Point start, Point end)
    {
        if (_arrowHeadDraft is null)
        {
            return;
        }

        Point tip = _arrowAtEnd ? end : start;
        Point tail = _arrowAtEnd ? start : end;
        double angle = Math.Atan2(tip.Y - tail.Y, tip.X - tail.X);
        const double head = 14;
        var hx1 = new Point(tip.X - (head * Math.Cos(angle - 0.45)), tip.Y - (head * Math.Sin(angle - 0.45)));
        var hx2 = new Point(tip.X - (head * Math.Cos(angle + 0.45)), tip.Y - (head * Math.Sin(angle + 0.45)));
        _arrowHeadDraft.Points.Clear();
        _arrowHeadDraft.Points.Add(hx1);
        _arrowHeadDraft.Points.Add(tip);
        _arrowHeadDraft.Points.Add(hx2);
    }

    private void ReleaseMosaicDraft()
    {
        _mosaicDraft?.ReleasePixels();
        _mosaicDraft = null;
        _mosaicDraftLast = -1;
    }

    private (int X, int Y) ToVirtual(Point windowPos) =>
        (
            _monitor.Bounds.X + (int)Math.Round(windowPos.X / _dipX),
            _monitor.Bounds.Y + (int)Math.Round(windowPos.Y / _dipY));

    private int ToPxX(double dip)
    {
        double width = _preview.ActualWidth > 1 ? _preview.ActualWidth : Math.Max(1, _original.Width * _dipX);
        return (int)Math.Round(dip * _original.Width / width);
    }

    private int ToPxY(double dip)
    {
        double height = _preview.ActualHeight > 1 ? _preview.ActualHeight : Math.Max(1, _original.Height * _dipY);
        return (int)Math.Round(dip * _original.Height / height);
    }

    private static SolidColorBrush Freeze(Color color)
    {
        var brush = new SolidColorBrush(color);
        brush.Freeze();
        return brush;
    }

    private sealed class ToolButton : Button
    {
        public ToolButton(AnnotateTool tool)
        {
            Tool = tool;
            Width = AnnotateToolbar.Cell;
            Height = AnnotateToolbar.Cell;
            Focusable = false;
            ToolTip = AnnotateToolbar.Tooltip(tool);
            Background = Brushes.Transparent;
            BorderThickness = new Thickness(0);
            Padding = new Thickness(0);
            var grid = new Grid();
            var glyph = AnnotateIcons.Glyph(tool);
            glyph.HorizontalAlignment = HorizontalAlignment.Center;
            glyph.VerticalAlignment = VerticalAlignment.Center;
            glyph.IsHitTestVisible = false;
            Dot = new Ellipse
            {
                Width = AnnotateToolbar.SelectedDot,
                Height = AnnotateToolbar.SelectedDot,
                Fill = DotInk,
                HorizontalAlignment = HorizontalAlignment.Right,
                VerticalAlignment = VerticalAlignment.Top,
                Margin = new Thickness(0, 4, 6, 0),
                Visibility = Visibility.Collapsed,
                IsHitTestVisible = false,
            };
            grid.Children.Add(glyph);
            grid.Children.Add(Dot);
            Content = grid;
            Template = BuildTemplate();
        }

        public AnnotateTool Tool { get; }
        public Ellipse Dot { get; }

        public void SetSelected(bool selected) =>
            Dot.Visibility = selected ? Visibility.Visible : Visibility.Collapsed;

        private static ControlTemplate BuildTemplate()
        {
            var template = new ControlTemplate(typeof(Button));
            var border = new FrameworkElementFactory(typeof(Border));
            border.Name = "Bd";
            border.SetValue(Border.BackgroundProperty, Brushes.Transparent);
            border.SetValue(Border.SnapsToDevicePixelsProperty, true);
            var presenter = new FrameworkElementFactory(typeof(ContentPresenter));
            presenter.SetValue(HorizontalAlignmentProperty, HorizontalAlignment.Center);
            presenter.SetValue(VerticalAlignmentProperty, VerticalAlignment.Center);
            border.AppendChild(presenter);
            template.VisualTree = border;
            var hover = new Trigger { Property = IsMouseOverProperty, Value = true };
            hover.Setters.Add(new Setter(Border.BackgroundProperty, ToolbarHover) { TargetName = "Bd" });
            var pressed = new Trigger { Property = IsPressedProperty, Value = true };
            pressed.Setters.Add(new Setter(Border.BackgroundProperty, ToolbarPressed) { TargetName = "Bd" });
            template.Triggers.Add(hover);
            template.Triggers.Add(pressed);
            return template;
        }
    }

    private interface IAnnotationOp
    {
        void Apply(PixelBuffer buffer, PixelBuffer original);
    }

    private sealed class RectOp(PixelRect rect, int thickness, byte b, byte g, byte r) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) =>
            PixelDraw.RectangleColor(buffer, rect, thickness, b, g, r, 255);
    }

    private sealed class EllipseOp(PixelRect rect, int thickness, byte b, byte g, byte r) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) =>
            PixelDraw.EllipseColor(buffer, rect, thickness, b, g, r, 255);
    }

    private sealed class MosaicStrokeOp(IReadOnlyList<(int X, int Y)> points, int brushRadius) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) =>
            PixelDraw.MosaicBrush(buffer, points, brushRadius);
    }

    private sealed class StrokeOp(
        IReadOnlyList<(int X, int Y)> points,
        int thickness,
        byte b,
        byte g,
        byte r,
        byte a,
        bool dashed = false) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original)
        {
            if (dashed && points.Count >= 2)
            {
                for (int i = 1; i < points.Count; i++)
                {
                    PixelDraw.DashedLineColor(
                        buffer,
                        points[i - 1].X,
                        points[i - 1].Y,
                        points[i].X,
                        points[i].Y,
                        thickness,
                        b,
                        g,
                        r,
                        a);
                }

                return;
            }

            PixelDraw.Polyline(buffer, points, thickness, b, g, r, a);
        }
    }

    private sealed class ArrowOp(int x1, int y1, int x2, int y2, int thickness, byte b, byte g, byte r) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) =>
            PixelDraw.ArrowColor(buffer, x1, y1, x2, y2, thickness, b, g, r, 255);
    }

    private sealed class EraseOp(IReadOnlyList<(int X, int Y)> points, int thickness) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original) =>
            PixelDraw.RestoreStroke(buffer, original, points, thickness);
    }

    private sealed class TextOp(int x, int y, string text, byte b, byte g, byte r, double fontSize) : IAnnotationOp
    {
        public void Apply(PixelBuffer buffer, PixelBuffer original)
        {
            var visual = new DrawingVisual();
            var formatted = new FormattedText(
                text,
                CultureInfo.CurrentUICulture,
                FlowDirection.LeftToRight,
                new Typeface("Segoe UI"),
                fontSize,
                new SolidColorBrush(Color.FromRgb(r, g, b)),
                1.0);
            using (DrawingContext dc = visual.RenderOpen())
            {
                dc.DrawImage(buffer.ToBitmapSource(), new Rect(0, 0, buffer.Width, buffer.Height));
                dc.DrawText(formatted, new Point(x, y));
            }

            var rtb = new RenderTargetBitmap(buffer.Width, buffer.Height, 96, 96, PixelFormats.Pbgra32);
            rtb.Render(visual);
            rtb.Freeze();
            PixelBuffer baked = PixelBuffer.FromBitmapSource(rtb);
            try
            {
                Buffer.BlockCopy(baked.Bgra, 0, buffer.Bgra, 0, Math.Min(baked.Bgra.Length, buffer.Bgra.Length));
            }
            finally
            {
                baked.ReleasePixels();
            }
        }
    }
}
