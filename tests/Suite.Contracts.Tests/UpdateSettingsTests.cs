using Suite.Contracts;

namespace Suite.Contracts.Tests;

public sealed class UpdateSettingsTests
{
    [Fact]
    public void Normalize_defaults_to_stable_and_placeholder_repo()
    {
        var u = new UpdateSettings { Channel = "weird", Owner = " ", Repo = "" };
        u.Normalize();
        Assert.Equal(UpdateSettings.ChannelStable, u.Channel);
        Assert.Equal("tsumon", u.Owner);
        Assert.Equal("suite", u.Repo);
        Assert.Equal("正式", u.ChannelDisplayZh());
    }

    [Fact]
    public void Preview_channel_display_is_chinese()
    {
        var u = new UpdateSettings { Channel = UpdateSettings.ChannelPreview };
        u.Normalize();
        Assert.Equal("预览", u.ChannelDisplayZh());
    }
}
