using Suite.Contracts;

namespace Suite.App;

/// <summary>
/// In-process taskbar appearance. Does not start TaskbarFx.exe or open a named pipe.
/// </summary>
internal sealed class TaskbarFxClient : IDisposable
{
    private readonly SemaphoreSlim _gate = new(1, 1);
    private readonly TaskbarFxRuntime _runtime = new();
    private bool _disposed;
    private bool _watching;

    public async Task<TaskbarFxStatusDto> ApplyAsync(TaskbarFxSettings settings, bool shutdownWhenDisabled)
    {
        using var gateTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(2));
        await _gate.WaitAsync(gateTimeout.Token).ConfigureAwait(false);
        try
        {
            if (!_watching)
            {
                _runtime.StartWatch();
                _watching = true;
            }

            TaskbarFxSettings snapshot = settings.Clone();
            if (!snapshot.Enabled && shutdownWhenDisabled)
            {
                snapshot.Mode = TaskbarAppearanceModes.Normal;
            }

            return await Task.Run(() => _runtime.Apply(snapshot)).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            return Unavailable("无法启用任务栏效果。截图和网速仍可用。");
        }
        catch (Exception)
        {
            return Unavailable(TaskbarFxCopy.InitFailed);
        }
        finally
        {
            _gate.Release();
        }
    }

    public Task DisconnectKeepAliveAsync()
    {
        DropWatch(reset: false);
        return Task.CompletedTask;
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        DropWatch(reset: false);
        _runtime.Dispose();
        _gate.Dispose();
    }

    private void DropWatch(bool reset)
    {
        if (reset)
        {
            try
            {
                TaskbarFxSettings off = new() { Enabled = false, Mode = TaskbarAppearanceModes.Normal };
                _runtime.Apply(off);
            }
            catch
            {
            }
        }
    }

    private static TaskbarFxStatusDto Unavailable(string message) => new()
    {
        Enabled = false,
        Path = TaskbarFxIpc.PathUnavailable,
        Mode = TaskbarAppearanceModes.Normal,
        LastError = message,
        Message = message,
    };
}