using Suite.Contracts;
using Suite.Platform;

namespace TaskbarFx.Host;

/// <summary>
/// Optional / deprecated helper EXE. Main path is Suite.exe loading TaskbarFx.Native.dll
/// in-process (architecture/DECISION-SINGLE-PROCESS.md). Kept for --reset and old installs.
/// </summary>
internal static class Program
{
    [STAThread]
    private static int Main(string[] args)
    {
        bool resetOnly = args.Any(a => string.Equals(a, "--reset", StringComparison.OrdinalIgnoreCase));

        if (resetOnly)
        {
            return RunReset();
        }

        if (!SingleInstanceGuard.TryAcquire(NativeConstants.TaskbarFxMutexName, out SingleInstanceGuard? guard))
        {
            return 0;
        }

        using (guard)
        {
            if (ProcessIntegrity.IsElevatedAdministrator())
            {
                HostLog.Write("refusing to run elevated; start as a standard user.");
            }

            using var runtime = new FxRuntime();
            runtime.TryLoadNative(out string? loadError);
            if (loadError is not null)
            {
                HostLog.Write(loadError);
            }

            runtime.StartWatch();
            using var server = new PipeServer(runtime);
            server.Start();
            HostLog.Write("listening on " + TaskbarFxPipe.GetPipeName());

            try
            {
                server.WaitAsync().GetAwaiter().GetResult();
            }
            catch (Exception ex)
            {
                HostLog.Write("server stop: " + ex.Message);
            }

            runtime.Disable();
            return 0;
        }
    }

    private static int RunReset()
    {
        using var runtime = new FxRuntime();
        if (!runtime.TryLoadNative(out string? error))
        {
            HostLog.Write(error ?? "native missing");
            return 2;
        }

        TaskbarFxStatusDto status = runtime.Disable();
        HostLog.Write("reset: " + (status.Message ?? status.LastError ?? "ok"));
        return string.IsNullOrEmpty(status.LastError) ? 0 : 1;
    }
}
