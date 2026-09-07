using System.IO;
using System.Text.Json;
using Suite.Contracts;

namespace Suite.App;

/// <summary>Dirty-exit detection + safe-mode degrade. INTERACTION-P2 §2.</summary>
public static class CrashGuard
{
    public const string MsgFxOff = "上次异常退出，已暂时关闭任务栏效果。";
    public const string MsgFloat = "上次异常退出，网速已改回桌面悬浮窗。";
    public const string MsgSafe = "已用安全设置启动。任务栏效果已暂时关闭；网速钉栏保持开启。";
    public const string MsgWriteFail = "没法写入安全设置，请检查磁盘权限。";

    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true,
    };

    public sealed class DegradeResult
    {
        public bool Degraded { get; init; }
        public bool SafeMode { get; init; }
        public string? Message { get; init; }
        public string LastStartLabel { get; init; } = "正常";
    }

    public static CrashGuardState Load()
    {
        try
        {
            if (!File.Exists(SettingsPaths.CrashGuardPath))
            {
                return new CrashGuardState();
            }

            string json = File.ReadAllText(SettingsPaths.CrashGuardPath);
            return JsonSerializer.Deserialize<CrashGuardState>(json, JsonOpts) ?? new CrashGuardState();
        }
        catch
        {
            return new CrashGuardState();
        }
    }

    public static bool TrySave(CrashGuardState state, out string? error)
    {
        error = null;
        try
        {
            Directory.CreateDirectory(SettingsPaths.DirectoryPath);
            File.WriteAllText(SettingsPaths.CrashGuardPath, JsonSerializer.Serialize(state, JsonOpts));
            return true;
        }
        catch (Exception ex)
        {
            error = MsgWriteFail + " " + ex.Message;
            return false;
        }
    }

    /// <summary>Call at process start before applying settings. May mutate <paramref name="settings"/>.</summary>
    public static DegradeResult OnStartup(AppSettings settings)
    {
        CrashGuardState state = Load();
        long now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        bool dirty = !state.CleanExit;
        bool recent = state.LastStartUnixMs > 0 && (now - state.LastStartUnixMs) < 30_000;
        int crashes = dirty ? state.RecentCrashCount + 1 : 0;
        if (dirty && recent)
        {
            crashes = Math.Max(crashes, state.RecentCrashCount + 1);
        }

        var messages = new List<string>();
        bool safe = dirty && crashes >= 4;
        bool degraded = false;

        if (safe)
        {
            // Safe-mode only kills TaskbarFx. Never force netspeed off the tray —
            // deploy Stop-Process / dirty exits must not demote Joe's embed to float.
            if (settings.TaskbarFx.Enabled)
            {
                settings.TaskbarFx.Enabled = false;
                degraded = true;
            }

            messages.Add(MsgSafe);
        }
        else if (dirty)
        {
            if (state.TaskbarFxWasEnabled && settings.TaskbarFx.Enabled && settings.Advanced.TaskbarFxAutoDisableOnFail)
            {
                settings.TaskbarFx.Enabled = false;
                messages.Add(MsgFxOff);
                degraded = true;
            }

            // Dirty exit: may disable TaskbarFx only. Never force netspeed off the tray.
            // Safe-mode (crashes >= 4) also leaves EmbedInTaskbar alone.
        }

        state.CleanExit = false; // mark running; clean exit clears
        state.TaskbarFxWasEnabled = settings.TaskbarFx.Enabled;
        state.NetSpeedEmbedWasOn = settings.NetSpeed.EmbedInTaskbar && settings.NetSpeed.Visible;
        state.LastStartUnixMs = now;
        state.RecentCrashCount = dirty ? crashes : 0;
        state.LastDegrade = messages.Count == 0 ? null : string.Join(" ", messages);
        TrySave(state, out _);

        return new DegradeResult
        {
            Degraded = degraded,
            SafeMode = safe,
            Message = messages.Count == 0 ? null : string.Join(" ", messages),
            LastStartLabel = degraded || safe ? "已降级" : (dirty ? "已降级" : "正常"),
        };
    }

    public static void MarkRunningFeatures(AppSettings settings)
    {
        CrashGuardState state = Load();
        state.CleanExit = false;
        state.TaskbarFxWasEnabled = settings.TaskbarFx.Enabled;
        state.NetSpeedEmbedWasOn = settings.NetSpeed.Visible && settings.NetSpeed.EmbedInTaskbar;
        TrySave(state, out _);
    }

    public static void MarkCleanExit()
    {
        CrashGuardState state = Load();
        state.CleanExit = true;
        state.RecentCrashCount = 0;
        TrySave(state, out _);
    }
}
