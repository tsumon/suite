namespace Suite.Contracts;

/// <summary>Update channel preferences. No tokens. INTERACTION-P2 §1.</summary>
public sealed class UpdateSettings
{
    public const string ChannelStable = "stable";
    public const string ChannelPreview = "preview";

    /// <summary>Wire: stable | preview. Default stable (正式).</summary>
    public string Channel { get; set; } = ChannelStable;

    /// <summary>GitHub owner/repo for release checks. Placeholder until publish.</summary>
    public string Owner { get; set; } = "tsumon";

    public string Repo { get; set; } = "suite";

    public UpdateSettings Clone() => new()
    {
        Channel = Channel,
        Owner = Owner,
        Repo = Repo,
    };

    public void Normalize()
    {
        if (!string.Equals(Channel, ChannelPreview, StringComparison.OrdinalIgnoreCase))
        {
            Channel = ChannelStable;
        }
        else
        {
            Channel = ChannelPreview;
        }

        Owner = string.IsNullOrWhiteSpace(Owner) ? "tsumon" : Owner.Trim();
        Repo = string.IsNullOrWhiteSpace(Repo) ? "suite" : Repo.Trim();
    }

    public string ChannelDisplayZh() =>
        string.Equals(Channel, ChannelPreview, StringComparison.OrdinalIgnoreCase) ? "预览" : "正式";
}
