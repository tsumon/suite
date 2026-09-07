using System.IO;
using System.IO.Pipes;
using Suite.Contracts;
using Suite.Platform;

namespace TaskbarFx.Host;

internal sealed class PipeServer : IDisposable
{
    private readonly FxRuntime _runtime;
    private readonly CancellationTokenSource _cts = new();
    private NamedPipeServerStream? _pipe;
    private readonly SemaphoreSlim _writeGate = new(1, 1);
    private Task? _loop;

    public PipeServer(FxRuntime runtime)
    {
        _runtime = runtime;
        _runtime.EventRaised += OnEvent;
    }

    public void Start() => _loop = Task.Run(() => RunAsync(_cts.Token));

    public Task WaitAsync() => _loop ?? Task.CompletedTask;

    public async Task StopAsync()
    {
        _cts.Cancel();
        NamedPipeServerStream? pipe = _pipe;
        try
        {
            pipe?.Dispose();
        }
        catch
        {
        }

        if (_loop is not null)
        {
            try
            {
                await _loop.ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
            }
            catch (IOException)
            {
            }
        }
    }

    public void Dispose()
    {
        _runtime.EventRaised -= OnEvent;
        _cts.Cancel();
        _pipe?.Dispose();
        _cts.Dispose();
        _writeGate.Dispose();
    }

    private async Task RunAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            NamedPipeServerStream? server = null;
            try
            {
                server = TaskbarFxPipe.CreateServer();
                _pipe = server;
                await server.WaitForConnectionAsync(cancellationToken).ConfigureAwait(false);
                if (!TaskbarFxPipe.TryValidatePeer(
                        server.SafePipeHandle,
                        weAreServer: true,
                        NativeConstants.SuiteExeFileName,
                        out string? peerError))
                {
                    HostLog.Write("peer rejected: " + peerError);
                    server.Disconnect();
                    continue;
                }

                await ServeConnectionAsync(server, cancellationToken).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                return;
            }
            catch (IOException ex)
            {
                HostLog.Write("pipe io: " + ex.Message);
            }
            catch (UnauthorizedAccessException ex)
            {
                HostLog.Write("pipe create failed (FirstPipeInstance / ACL): " + ex.Message);
                return;
            }
            finally
            {
                if (server is not null)
                {
                    try
                    {
                        server.Dispose();
                    }
                    catch
                    {
                    }
                }

                if (ReferenceEquals(_pipe, server))
                {
                    _pipe = null;
                }
            }
        }
    }

    private async Task ServeConnectionAsync(NamedPipeServerStream server, CancellationToken cancellationToken)
    {
        while (server.IsConnected && !cancellationToken.IsCancellationRequested)
        {
            using var callCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            callCts.CancelAfter(TaskbarFxIpc.CallTimeout);
            string? json;
            try
            {
                json = await TaskbarFxIpc.ReadFrameAsync(server, callCts.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
            {
                continue;
            }

            if (json is null)
            {
                return;
            }

            TaskbarFxEnvelope request;
            try
            {
                request = TaskbarFxIpc.Deserialize(json);
            }
            catch (Exception ex)
            {
                await WriteAsync(server, TaskbarFxIpc.Fail("", "Bad JSON: " + ex.Message), cancellationToken)
                    .ConfigureAwait(false);
                continue;
            }

            TaskbarFxEnvelope reply = Handle(request);
            await WriteAsync(server, reply, cancellationToken).ConfigureAwait(false);
            if (string.Equals(request.Op, TaskbarFxIpc.OpShutdown, StringComparison.OrdinalIgnoreCase))
            {
                _cts.Cancel();
                return;
            }
        }
    }

    private TaskbarFxEnvelope Handle(TaskbarFxEnvelope request)
    {
        string id = request.Id ?? "";
        try
        {
            switch (request.Op)
            {
                case TaskbarFxIpc.OpPing:
                    return TaskbarFxIpc.Ok(id, _runtime.Status);
                case TaskbarFxIpc.OpGetStatus:
                    return TaskbarFxIpc.Ok(id, _runtime.Status);
                case TaskbarFxIpc.OpEnable:
                    return Reply(id, _runtime.Enable());
                case TaskbarFxIpc.OpDisable:
                    return TaskbarFxIpc.Ok(id, _runtime.Disable());
                case TaskbarFxIpc.OpSetAppearance:
                    if (!TaskbarAppearanceModes.TryParse(request.Mode, out TaskbarAppearanceMode mode))
                    {
                        return TaskbarFxIpc.Fail(id, "Unknown mode.", _runtime.Status);
                    }

                    uint argb = request.Argb ?? TaskbarFxSettings.DefaultArgb;
                    return Reply(id, _runtime.SetAppearance(mode, argb));
                case TaskbarFxIpc.OpShutdown:
                    _runtime.Disable();
                    return TaskbarFxIpc.Ok(id, _runtime.Status);
                default:
                    return TaskbarFxIpc.Fail(id, "Unknown op " + request.Op, _runtime.Status);
            }
        }
        catch (Exception ex)
        {
            return TaskbarFxIpc.Fail(id, ex.Message, _runtime.Status);
        }
    }

    private static TaskbarFxEnvelope Reply(string id, TaskbarFxStatusDto status)
    {
        bool unavailable = string.Equals(status.Path, TaskbarFxIpc.PathUnavailable, StringComparison.OrdinalIgnoreCase);
        if (unavailable && !string.IsNullOrEmpty(status.LastError))
        {
            return TaskbarFxIpc.Fail(id, status.LastError, status);
        }

        return TaskbarFxIpc.Ok(id, status);
    }

    private void OnEvent(object? sender, TaskbarFxEnvelope envelope)
    {
        NamedPipeServerStream? pipe = _pipe;
        if (pipe is null || !pipe.IsConnected)
        {
            return;
        }

        _ = WriteAsync(pipe, envelope, CancellationToken.None);
    }

    private async Task WriteAsync(NamedPipeServerStream pipe, TaskbarFxEnvelope envelope, CancellationToken cancellationToken)
    {
        await _writeGate.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(TaskbarFxIpc.CallTimeout);
            await TaskbarFxIpc.WriteFrameAsync(pipe, TaskbarFxIpc.Serialize(envelope), timeout.Token)
                .ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            HostLog.Write("pipe write: " + ex.Message);
        }
        finally
        {
            _writeGate.Release();
        }
    }
}
