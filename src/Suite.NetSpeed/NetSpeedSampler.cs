namespace Suite.NetSpeed;

public sealed class NetSpeedSampler : IDisposable
{
    private readonly IInterfaceTable _table;
    private readonly TimeSpan _interval;
    private readonly object _gate = new();
    private CancellationTokenSource? _cts;
    private Task? _loop;
    private InterfaceSnapshot? _previous;
    private IReadOnlyList<InterfaceSnapshot>? _previousTable;
    private DateTime _previousUtc;
    private uint? _preferredIfIndex;
    private string? _preferredAlias;
    private bool _disposed;

    public NetSpeedSampler(IInterfaceTable table, TimeSpan? interval = null)
    {
        _table = table;
        _interval = interval ?? TimeSpan.FromSeconds(1);
    }

    public event EventHandler<RateSample>? Sampled;

    public void SetPreferredAdapter(uint? ifIndex, string? alias)
    {
        lock (_gate)
        {
            _preferredIfIndex = ifIndex;
            _preferredAlias = alias;
            _previous = null;
            _previousTable = null;
        }
    }

    public IReadOnlyList<InterfaceSnapshot> ListAdapters() => _table.GetTable();

    public void ResetAfterResume()
    {
        lock (_gate)
        {
            _previous = null;
            _previousTable = null;
        }
    }

    public void Start()
    {
        lock (_gate)
        {
            ObjectDisposedException.ThrowIf(_disposed, this);
            if (_loop is { IsCompleted: false })
            {
                return;
            }

            _cts = new CancellationTokenSource();
            _loop = Task.Run(() => LoopAsync(_cts.Token));
        }
    }

    public void Stop()
    {
        CancellationTokenSource? cts;
        Task? loop;
        lock (_gate)
        {
            cts = _cts;
            loop = _loop;
            _cts = null;
            _loop = null;
        }

        cts?.Cancel();
        try
        {
            loop?.Wait(TimeSpan.FromSeconds(2));
        }
        catch (AggregateException)
        {
        }

        cts?.Dispose();
    }

    public void Dispose()
    {
        lock (_gate)
        {
            if (_disposed)
            {
                return;
            }

            _disposed = true;
        }

        Stop();
    }

    private async Task LoopAsync(CancellationToken token)
    {
        using var timer = new PeriodicTimer(_interval);
        SampleOnce();
        try
        {
            while (await timer.WaitForNextTickAsync(token).ConfigureAwait(false))
            {
                SampleOnce();
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private void SampleOnce()
    {
        IReadOnlyList<InterfaceSnapshot> table;
        try
        {
            table = _table.GetTable();
        }
        catch (Exception)
        {
            Sampled?.Invoke(this, new RateSample
            {
                IfIndex = 0,
                Alias = "",
                ReceiveBytesPerSecond = 0,
                SendBytesPerSecond = 0,
                AdapterMissing = true,
            });
            return;
        }

        uint? preferredIndex;
        string? preferredAlias;
        InterfaceSnapshot? previous;
        IReadOnlyList<InterfaceSnapshot>? previousTable;
        DateTime previousUtc;
        lock (_gate)
        {
            preferredIndex = _preferredIfIndex;
            preferredAlias = _preferredAlias;
            previous = _previous;
            previousTable = _previousTable;
            previousUtc = _previousUtc;
        }

        InterfaceSnapshot? resolved = AdapterResolver.Resolve(table, preferredIndex, preferredAlias, previousTable);
        InterfaceSnapshot? current = resolved is null ? null : (_table.GetEntry(resolved.IfIndex) ?? resolved);
        if (current is null)
        {
            lock (_gate)
            {
                _previous = null;
            }

            Sampled?.Invoke(this, new RateSample
            {
                IfIndex = preferredIndex ?? 0,
                Alias = preferredAlias ?? "",
                ReceiveBytesPerSecond = 0,
                SendBytesPerSecond = 0,
                AdapterMissing = true,
            });
            return;
        }

        DateTime now = DateTime.UtcNow;
        double receive = 0;
        double send = 0;
        if (previous is not null)
        {
            double elapsed = (now - previousUtc).TotalSeconds;
            RateCalculator.TryCompute(previous, current, elapsed, out receive, out send);
        }

        lock (_gate)
        {
            _previous = current;
            _previousTable = table.ToArray();
            _previousUtc = now;
            _preferredIfIndex = current.IfIndex;
            _preferredAlias = current.Alias;
        }

        Sampled?.Invoke(this, new RateSample
        {
            IfIndex = current.IfIndex,
            Alias = current.Alias,
            ReceiveBytesPerSecond = receive,
            SendBytesPerSecond = send,
            AdapterMissing = false,
        });
    }
}
