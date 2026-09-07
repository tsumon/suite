using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Security.Principal;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace Suite.Platform;

/// <summary>
/// Named-pipe identity for Suite ↔ TaskbarFx.
/// Name includes the current user SID + session so it is not a single globally guessable string.
/// Server uses PipeOptions.CurrentUserOnly + FirstPipeInstance. No System.IO.AccessControl package (NU1510).
/// Release builds have no "debugger / Everyone" switch — that would be fail-open (AUDIT P1-1).
/// </summary>
public static class TaskbarFxPipe
{
    public static string GetPipeName()
    {
        using var identity = WindowsIdentity.GetCurrent();
        string sid = identity.User?.Value ?? "unknown";
        return FormatPipeName(sid, ProcessSessionId());
    }

    public static string FormatPipeName(string userSid, int sessionId)
    {
        string sid = (userSid ?? "").Replace('\\', '-').Replace('/', '-');
        if (string.IsNullOrWhiteSpace(sid))
        {
            sid = "unknown";
        }

        return $"{NativeConstants.TaskbarFxPipePrefix}.{sid}.s{sessionId}";
    }

    /// <summary>
    /// Server: CurrentUserOnly + FirstPipeInstance.
    /// Single-process path does not add PipeSecurity / AccessControl (NU1510).
    /// </summary>
    public static NamedPipeServerStream CreateServer()
    {
        string name = GetPipeName();
        var options = PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly | PipeOptions.FirstPipeInstance;
        var server = new NamedPipeServerStream(
            name,
            PipeDirection.InOut,
            maxNumberOfServerInstances: 1,
            PipeTransmissionMode.Byte,
            options,
            inBufferSize: 4096,
            outBufferSize: 4096);

        TightenAcl(server);
        return server;
    }

    public static NamedPipeClientStream CreateClient()
    {
        return new NamedPipeClientStream(
            ".",
            GetPipeName(),
            PipeDirection.InOut,
            PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly);
    }

    public static void TightenAcl(NamedPipeServerStream server)
    {
        // Single-process path no longer relies on pipe ACL hardening.
        // NamedPipeServerStream is created with PipeOptions.CurrentUserOnly.
        _ = server;
    }

    /// <summary>
    /// After connect: peer must be the expected EXE in the same directory as this process.
    /// If this process is Authenticode-signed, the peer must have the same signer subject.
    /// Unsigned dev builds skip the signer check so Joe can run locally (AUDIT P1-7 is for packages that leave the machine).
    /// </summary>
    public static bool TryValidatePeer(
        SafePipeHandle pipeHandle,
        bool weAreServer,
        string expectedFileName,
        out string? error)
    {
        error = null;
        IntPtr handle = pipeHandle.DangerousGetHandle();
        uint peerPid;
        bool ok = weAreServer
            ? TaskbarNative.GetNamedPipeClientProcessId(handle, out peerPid)
            : TaskbarNative.GetNamedPipeServerProcessId(handle, out peerPid);
        if (!ok || peerPid == 0)
        {
            error = "Unable to read the pipe peer process id.";
            return false;
        }

        if (peerPid == (uint)Environment.ProcessId)
        {
            error = "Pipe peer resolved to this process.";
            return false;
        }

        if (!TryGetImagePath(peerPid, out string? peerPath) || string.IsNullOrWhiteSpace(peerPath))
        {
            error = "Unable to read the pipe peer image path.";
            return false;
        }

        string selfDir = Path.GetFullPath(AppContext.BaseDirectory)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        string peerDir = Path.GetFullPath(Path.GetDirectoryName(peerPath) ?? "")
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        if (!string.Equals(selfDir, peerDir, StringComparison.OrdinalIgnoreCase))
        {
            error = "Pipe peer is not in the same directory.";
            return false;
        }

        string peerName = Path.GetFileName(peerPath);
        if (!string.Equals(peerName, expectedFileName, StringComparison.OrdinalIgnoreCase))
        {
            error = "Pipe peer executable name does not match " + expectedFileName + ".";
            return false;
        }

        bool weElevated = ProcessIntegrity.IsElevatedAdministrator();
        bool theyElevated = IsProcessElevated(peerPid);
        if (weElevated != theyElevated)
        {
            error = "Pipe peer elevation does not match this process.";
            return false;
        }

        string selfPath = Environment.ProcessPath ?? "";
        if (TryGetSignerSubject(selfPath, out string? selfSigner)
            && !string.IsNullOrWhiteSpace(selfSigner))
        {
            if (!TryGetSignerSubject(peerPath, out string? peerSigner)
                || !string.Equals(selfSigner, peerSigner, StringComparison.OrdinalIgnoreCase))
            {
                error = "Pipe peer Authenticode signer does not match.";
                return false;
            }
        }

        return true;
    }

    public static bool TryGetImagePath(uint processId, out string? path)
    {
        path = null;
        IntPtr process = TaskbarNative.OpenProcess(TaskbarNative.ProcessQueryLimitedInformation, false, processId);
        if (process == IntPtr.Zero)
        {
            return false;
        }

        try
        {
            var buffer = new StringBuilder(1024);
            uint size = (uint)buffer.Capacity;
            if (!TaskbarNative.QueryFullProcessImageNameW(process, 0, buffer, ref size))
            {
                return false;
            }

            path = buffer.ToString();
            return !string.IsNullOrWhiteSpace(path);
        }
        finally
        {
            TaskbarNative.CloseHandle(process);
        }
    }

    private static bool IsProcessElevated(uint processId)
    {
        IntPtr process = TaskbarNative.OpenProcess(TaskbarNative.ProcessQueryLimitedInformation, false, processId);
        if (process == IntPtr.Zero)
        {
            return false;
        }

        IntPtr token = IntPtr.Zero;
        try
        {
            if (!TaskbarNative.OpenProcessToken(process, TaskbarNative.TokenQuery, out token))
            {
                return false;
            }

            const int tokenElevation = 20;
            if (!TaskbarNative.GetTokenInformation(
                    token,
                    tokenElevation,
                    out TaskbarNative.TokenElevation elevation,
                    Marshal.SizeOf<TaskbarNative.TokenElevation>(),
                    out _))
            {
                return false;
            }

            return elevation.TokenIsElevated != 0;
        }
        finally
        {
            if (token != IntPtr.Zero)
            {
                TaskbarNative.CloseHandle(token);
            }

            TaskbarNative.CloseHandle(process);
        }
    }

    private static bool TryGetSignerSubject(string path, out string? subject)
    {
        subject = null;
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path))
        {
            return false;
        }

        try
        {
#pragma warning disable SYSLIB0057 // signed-file helper; peer check is best-effort for signed releases
            using var cert = X509Certificate.CreateFromSignedFile(path);
#pragma warning restore SYSLIB0057
            subject = cert.Subject;
            return !string.IsNullOrWhiteSpace(subject);
        }
        catch (CryptographicException)
        {
            return false;
        }
        catch
        {
            return false;
        }
    }

    private static int ProcessSessionId()
    {
        try
        {
            return System.Diagnostics.Process.GetCurrentProcess().SessionId;
        }
        catch
        {
            return 0;
        }
    }
}
