using System.Windows;
using System.Windows.Threading;
using Suite.Platform;

namespace Suite.App;

public partial class App : System.Windows.Application
{
    private SingleInstanceGuard? _instance;
    private AppController? _controller;

    private void OnStartup(object sender, StartupEventArgs e)
    {
        DispatcherUnhandledException += (_, args) =>
        {
            SuiteLog.Error("DispatcherUnhandledException", args.Exception);
            args.Handled = true;
        };
        AppDomain.CurrentDomain.UnhandledException += (_, args) =>
        {
            if (args.ExceptionObject is Exception ex)
            {
                SuiteLog.Error("UnhandledException", ex);
            }
            else
            {
                SuiteLog.Info("UnhandledException non-Exception: " + (args.ExceptionObject?.ToString() ?? ""));
            }
        };
        TaskScheduler.UnobservedTaskException += (_, args) =>
        {
            SuiteLog.Error("UnobservedTaskException", args.Exception);
            args.SetObserved();
        };

        if (!SingleInstanceGuard.TryAcquire(out _instance))
        {
            Shutdown();
            return;
        }

        _controller = new AppController();
        _controller.Start();
    }

    private void OnExit(object sender, ExitEventArgs e)
    {
        try
        {
            CrashGuard.MarkCleanExit();
        }
        catch
        {
        }

        _controller?.Dispose();
        _instance?.Dispose();
    }
}
