using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Suite.Contracts;
using Suite.Platform;

namespace TaskbarFx.Host;

/// <summary>
/// Loads TaskbarFx.Native.dll from this EXE's directory (absolute path).
/// Optional host only. Suite loads the same DLL in-process via TaskbarFxNativeBridge.
/// </summary>
internal sealed class NativeBridge : IDisposable
{
    private IntPtr _library;
    private bool _disposed;

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int ApplyFn(int mode, uint argb);

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int ResetFn();

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int ProbeFn();

    [UnmanagedFunctionPointer(CallingConvention.StdCall, CharSet = CharSet.Unicode)]
    private delegate int LastMessageFn(StringBuilder buffer, int cch);

    private ApplyFn? _apply;
    private ResetFn? _reset;
    private ProbeFn? _probe;
    private LastMessageFn? _lastMessage;

    public bool IsLoaded => _library != IntPtr.Zero;

    public string? LoadError { get; private set; }

    public bool TryLoad()
    {
        if (IsLoaded)
        {
            return true;
        }

        string path = Path.Combine(AppContext.BaseDirectory, NativeConstants.TaskbarFxNativeFileName);
        if (!File.Exists(path))
        {
            LoadError = "TaskbarFx.Native.dll is missing. Build the C++ project in Visual Studio (see plans/P1B-HANDOFF.md).";
            return false;
        }

        try
        {
            _library = NativeLibrary.Load(path);
            _apply = GetExport<ApplyFn>("TaskbarFxApply");
            _reset = GetExport<ResetFn>("TaskbarFxReset");
            _probe = GetExport<ProbeFn>("TaskbarFxProbeKind");
            _lastMessage = GetExport<LastMessageFn>("TaskbarFxLastMessage");
            LoadError = null;
            return true;
        }
        catch (Exception ex)
        {
            LoadError = "Failed to load TaskbarFx.Native.dll: " + ex.Message;
            return false;
        }
    }

    public TaskbarKind Probe()
    {
        if (_probe is null)
        {
            return TaskbarKind.NotFound;
        }

        return _probe() switch
        {
            1 => TaskbarKind.ClassicWin32,
            2 => TaskbarKind.ModernXaml,
            _ => TaskbarKind.NotFound,
        };
    }

    public int Apply(TaskbarAppearanceMode mode, uint argb)
    {
        if (_apply is null)
        {
            return 1;
        }

        return _apply((int)mode, argb);
    }

    public int Reset()
    {
        if (_reset is null)
        {
            return 1;
        }

        return _reset();
    }

    public string LastMessage()
    {
        if (_lastMessage is null)
        {
            return LoadError ?? "";
        }

        var buffer = new StringBuilder(512);
        _lastMessage(buffer, buffer.Capacity);
        return buffer.ToString();
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        if (_library != IntPtr.Zero)
        {
            NativeLibrary.Free(_library);
            _library = IntPtr.Zero;
        }
    }

    private T GetExport<T>(string name) where T : Delegate
    {
        if (!NativeLibrary.TryGetExport(_library, name, out IntPtr proc))
        {
            throw new EntryPointNotFoundException(name);
        }

        return Marshal.GetDelegateForFunctionPointer<T>(proc);
    }
}
