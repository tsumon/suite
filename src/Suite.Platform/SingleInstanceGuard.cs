namespace Suite.Platform;

public sealed class SingleInstanceGuard : IDisposable
{
    private readonly Mutex _mutex;
    private bool _disposed;

    private SingleInstanceGuard(Mutex mutex) => _mutex = mutex;

    public static bool TryAcquire(out SingleInstanceGuard? guard) =>
        TryAcquire(NativeConstants.SingleInstanceMutexName, out guard);

    public static bool TryAcquire(string mutexName, out SingleInstanceGuard? guard)
    {
        var mutex = new Mutex(initiallyOwned: true, mutexName, out bool createdNew);
        try
        {
            if (!createdNew)
            {
                createdNew = mutex.WaitOne(0);
            }
        }
        catch (AbandonedMutexException)
        {
            createdNew = true;
        }

        if (!createdNew)
        {
            mutex.Dispose();
            guard = null;
            return false;
        }

        guard = new SingleInstanceGuard(mutex);
        return true;
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        try
        {
            _mutex.ReleaseMutex();
        }
        catch (ApplicationException)
        {
        }

        _mutex.Dispose();
    }
}
