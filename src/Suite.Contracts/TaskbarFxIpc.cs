using System.Buffers.Binary;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Suite.Contracts;

/// <summary>
/// Named-pipe protocol: 4-byte little-endian length prefix + UTF-8 JSON.
/// Suite is client, TaskbarFx is server. Every call should be bounded by <see cref="CallTimeout"/>.
/// </summary>
public static class TaskbarFxIpc
{
    public const int ProtocolVersion = 1;
    public const int MaxFrameBytes = 64 * 1024;
    public static readonly TimeSpan CallTimeout = TimeSpan.FromSeconds(2);

    public const string OpEnable = "Enable";
    public const string OpDisable = "Disable";
    public const string OpSetAppearance = "SetAppearance";
    public const string OpGetStatus = "GetStatus";
    public const string OpShutdown = "Shutdown";
    public const string OpPing = "Ping";
    public const string OpEvent = "Event";

    public const string PathWin10Swca = "win10-swca";
    public const string PathWin11Xaml = "win11-xaml";
    public const string PathUnavailable = "unavailable";

    public const string KindUnavailable = "unavailable";
    public const string KindDegraded = "degraded";
    public const string KindExplorerRestart = "explorer-restart";
    public const string KindReady = "ready";

    public static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        WriteIndented = false,
    };

    public static TaskbarFxEnvelope Request(string op, string? id = null) => new()
    {
        V = ProtocolVersion,
        Id = id ?? Guid.NewGuid().ToString("N"),
        Op = op,
    };

    public static TaskbarFxEnvelope AppearanceRequest(string mode, uint argb) => new()
    {
        V = ProtocolVersion,
        Id = Guid.NewGuid().ToString("N"),
        Op = OpSetAppearance,
        Mode = mode,
        Argb = argb,
    };

    public static TaskbarFxEnvelope Ok(string id, TaskbarFxStatusDto? status = null) => new()
    {
        V = ProtocolVersion,
        Id = id,
        Ok = true,
        Status = status,
    };

    public static TaskbarFxEnvelope Fail(string id, string error, TaskbarFxStatusDto? status = null) => new()
    {
        V = ProtocolVersion,
        Id = id,
        Ok = false,
        Error = error,
        Status = status,
    };

    public static TaskbarFxEnvelope Event(string kind, string? detail, TaskbarFxStatusDto? status = null) => new()
    {
        V = ProtocolVersion,
        Id = Guid.NewGuid().ToString("N"),
        Op = OpEvent,
        Kind = kind,
        Detail = detail,
        Status = status,
    };

    public static string Serialize(TaskbarFxEnvelope envelope) =>
        JsonSerializer.Serialize(envelope, JsonOptions);

    public static TaskbarFxEnvelope Deserialize(string json)
    {
        TaskbarFxEnvelope envelope = JsonSerializer.Deserialize<TaskbarFxEnvelope>(json, JsonOptions)
            ?? throw new InvalidOperationException("Empty TaskbarFx IPC payload.");
        if (envelope.V <= 0)
        {
            envelope.V = ProtocolVersion;
        }

        envelope.Id ??= "";
        envelope.Op ??= "";
        return envelope;
    }

    public static async Task WriteFrameAsync(Stream stream, string json, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(stream);
        byte[] utf8 = Encoding.UTF8.GetBytes(json);
        if (utf8.Length == 0 || utf8.Length > MaxFrameBytes)
        {
            throw new InvalidOperationException("TaskbarFx IPC frame size is invalid.");
        }

        byte[] header = new byte[4];
        BinaryPrimitives.WriteInt32LittleEndian(header, utf8.Length);
        await stream.WriteAsync(header, cancellationToken).ConfigureAwait(false);
        await stream.WriteAsync(utf8, cancellationToken).ConfigureAwait(false);
        await stream.FlushAsync(cancellationToken).ConfigureAwait(false);
    }

    public static async Task<string?> ReadFrameAsync(Stream stream, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(stream);
        byte[] header = new byte[4];
        if (!await ReadExactAsync(stream, header, cancellationToken).ConfigureAwait(false))
        {
            return null;
        }

        int length = BinaryPrimitives.ReadInt32LittleEndian(header);
        if (length <= 0 || length > MaxFrameBytes)
        {
            throw new InvalidOperationException("TaskbarFx IPC frame length is out of range.");
        }

        byte[] body = new byte[length];
        if (!await ReadExactAsync(stream, body, cancellationToken).ConfigureAwait(false))
        {
            return null;
        }

        return Encoding.UTF8.GetString(body);
    }

    private static async Task<bool> ReadExactAsync(Stream stream, byte[] buffer, CancellationToken cancellationToken)
    {
        int offset = 0;
        while (offset < buffer.Length)
        {
            int read = await stream.ReadAsync(buffer.AsMemory(offset, buffer.Length - offset), cancellationToken)
                .ConfigureAwait(false);
            if (read == 0)
            {
                return false;
            }

            offset += read;
        }

        return true;
    }
}

public sealed class TaskbarFxEnvelope
{
    public int V { get; set; } = TaskbarFxIpc.ProtocolVersion;
    public string Id { get; set; } = "";
    public string Op { get; set; } = "";
    public string? Mode { get; set; }
    public uint? Argb { get; set; }
    public bool? Ok { get; set; }
    public string? Error { get; set; }
    public string? Kind { get; set; }
    public string? Detail { get; set; }
    public TaskbarFxStatusDto? Status { get; set; }
}

public sealed class TaskbarFxStatusDto
{
    public bool Enabled { get; set; }
    public string Path { get; set; } = TaskbarFxIpc.PathUnavailable;
    public string Mode { get; set; } = TaskbarAppearanceModes.Normal;
    public uint Argb { get; set; }
    public string? LastError { get; set; }
    public string? Message { get; set; }
}
