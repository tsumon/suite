using System.IO;
using System.Net.Http;
using System.Reflection;
using System.Text.Json;
using Suite.Contracts;

namespace Suite.App;

/// <summary>GitHub releases check. Manual only — no auto-force install. INTERACTION-P2 §1.</summary>
public static class UpdateChecker
{
    public const string Checking = "正在检查更新…";
    public const string UpToDate = "已是最新版本。";
    public const string Found = "发现新版本。";
    public const string Downloading = "正在下载更新…";
    public const string FailNetwork = "检查更新失败，请稍后再试。";
    public const string FailEmpty = "这个通道暂时没有可用更新。";
    public const string FailDownload = "下载更新失败，请稍后再试。";
    public const string FailNotWired = "更新检查尚未在本机启用。";

    public sealed class CheckResult
    {
        public bool Ok { get; init; }
        public string Message { get; init; } = "";
        public string? Version { get; init; }
        public string? HtmlUrl { get; init; }
        public string? ChannelZh { get; init; }
        public bool HasUpdate { get; init; }
    }

    /// <summary>Short x.y.z for UI and compare (InformationalVersion / FileVersion preferred).</summary>
    public static string LocalVersion
    {
        get
        {
            try
            {
                Assembly asm = Assembly.GetExecutingAssembly();
                string? info = asm.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion;
                if (!string.IsNullOrWhiteSpace(info))
                {
                    // Strip any +git metadata from InformationalVersion.
                    int plus = info.IndexOf('+');
                    if (plus >= 0)
                    {
                        info = info[..plus];
                    }

                    return NormalizeVersion(info);
                }

                string? file = asm.GetCustomAttribute<AssemblyFileVersionAttribute>()?.Version;
                if (!string.IsNullOrWhiteSpace(file))
                {
                    return NormalizeVersion(file);
                }

                return NormalizeVersion(asm.GetName().Version?.ToString());
            }
            catch
            {
                return "0.0.0";
            }
        }
    }

    public static string LocalVersionDisplay => LocalVersion;

    public static async Task<CheckResult> CheckAsync(UpdateSettings settings, CancellationToken ct = default)
    {
        settings.Normalize();
        string owner = settings.Owner;
        string repo = settings.Repo;
        if (string.IsNullOrWhiteSpace(owner) || string.IsNullOrWhiteSpace(repo))
        {
            return new CheckResult { Ok = false, Message = FailNotWired };
        }

        try
        {
            using var client = new HttpClient { Timeout = TimeSpan.FromSeconds(12) };
            client.DefaultRequestHeaders.UserAgent.ParseAdd("Suite/" + LocalVersion);
            client.DefaultRequestHeaders.Accept.ParseAdd("application/vnd.github+json");
            string url = $"https://api.github.com/repos/{owner}/{repo}/releases?per_page=10";
            using HttpResponseMessage response = await client.GetAsync(url, ct).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode)
            {
                return new CheckResult { Ok = false, Message = FailNetwork };
            }

            await using Stream stream = await response.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
            using JsonDocument doc = await JsonDocument.ParseAsync(stream, cancellationToken: ct).ConfigureAwait(false);
            if (doc.RootElement.ValueKind != JsonValueKind.Array || doc.RootElement.GetArrayLength() == 0)
            {
                return new CheckResult { Ok = false, Message = FailEmpty };
            }

            bool wantPrerelease = string.Equals(settings.Channel, UpdateSettings.ChannelPreview, StringComparison.OrdinalIgnoreCase);
            JsonElement? match = null;
            foreach (JsonElement rel in doc.RootElement.EnumerateArray())
            {
                bool pre = rel.TryGetProperty("prerelease", out JsonElement p) && p.GetBoolean();
                bool draft = rel.TryGetProperty("draft", out JsonElement d) && d.GetBoolean();
                if (draft)
                {
                    continue;
                }

                if (wantPrerelease ? pre : !pre)
                {
                    match = rel;
                    break;
                }
            }

            // Preview may fall back to newest including pre; stable stays non-pre only.
            if (match is null && wantPrerelease)
            {
                foreach (JsonElement rel in doc.RootElement.EnumerateArray())
                {
                    bool draft = rel.TryGetProperty("draft", out JsonElement d) && d.GetBoolean();
                    if (!draft)
                    {
                        match = rel;
                        break;
                    }
                }
            }

            if (match is null)
            {
                return new CheckResult { Ok = false, Message = FailEmpty };
            }

            string tag = match.Value.TryGetProperty("tag_name", out JsonElement tagEl) ? tagEl.GetString() ?? "" : "";
            string html = match.Value.TryGetProperty("html_url", out JsonElement htmlEl) ? htmlEl.GetString() ?? "" : "";
            string remote = NormalizeVersion(tag);
            string local = NormalizeVersion(LocalVersion);
            bool newer = CompareSemVer(remote, local) > 0;
            string channelZh = settings.ChannelDisplayZh();
            if (!newer)
            {
                return new CheckResult
                {
                    Ok = true,
                    Message = UpToDate + "（当前 " + local + "，远端 " + tag + "）",
                    Version = tag,
                    HtmlUrl = html,
                    ChannelZh = channelZh,
                    HasUpdate = false,
                };
            }

            return new CheckResult
            {
                Ok = true,
                Message = Found + "（当前 " + local + " → " + channelZh + " " + tag + "）",
                Version = tag,
                HtmlUrl = html,
                ChannelZh = channelZh,
                HasUpdate = true,
            };
        }
        catch (OperationCanceledException)
        {
            return new CheckResult { Ok = false, Message = FailNetwork };
        }
        catch (HttpRequestException)
        {
            return new CheckResult { Ok = false, Message = FailNetwork };
        }
        catch
        {
            return new CheckResult { Ok = false, Message = FailNetwork };
        }
    }

    public static string NormalizeVersion(string? raw)
    {
        if (string.IsNullOrWhiteSpace(raw))
        {
            return "0.0.0";
        }

        string s = raw.Trim();
        if (s.StartsWith('v') || s.StartsWith('V'))
        {
            s = s[1..];
        }

        int plus = s.IndexOfAny(['-', '+']);
        if (plus >= 0)
        {
            s = s[..plus];
        }

        return s;
    }

    /// <summary>Positive if a &gt; b.</summary>
    public static int CompareSemVer(string a, string b)
    {
        int[] pa = ParseParts(a);
        int[] pb = ParseParts(b);
        for (int i = 0; i < 3; i++)
        {
            int c = pa[i].CompareTo(pb[i]);
            if (c != 0)
            {
                return c;
            }
        }

        return 0;
    }

    private static int[] ParseParts(string v)
    {
        string[] bits = v.Split('.', StringSplitOptions.RemoveEmptyEntries);
        int[] parts = [0, 0, 0];
        for (int i = 0; i < Math.Min(3, bits.Length); i++)
        {
            _ = int.TryParse(bits[i], out parts[i]);
        }

        return parts;
    }
}
