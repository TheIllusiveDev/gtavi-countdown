namespace GtaviCountdown.Tests;

public class SettingsTests
{
    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("null")]
    [InlineData("42")]
    [InlineData("\"text\"")]
    [InlineData("[]")]
    [InlineData("true")]
    [InlineData("{not json")]
    public void Sanitize_ReturnsTheDefaultsForMissingOrMalformedInput(string? input) =>
        Assert.Equal(SettingsData.Defaults, SettingsCodec.Sanitize(input));

    [Fact]
    public void Sanitize_KeepsValidFields()
    {
        var stored = new SettingsData("Pacific/Auckland", true, Theme.Midnight, EffectLevel.Low, true, false, true);
        Assert.Equal(stored, SettingsCodec.Sanitize(SettingsCodec.Serialize(stored)));
    }

    [Fact]
    public void Sanitize_ReadsTheStoredFormOfTheWebVersion()
    {
        const string json = """
            {"timeZone":"Pacific/Auckland","showMilliseconds":true,"theme":"midnight","effects":"low",
             "notify":true,"networkTime":false,"alwaysOnTop":true}
            """;
        Assert.Equal(
            new SettingsData("Pacific/Auckland", true, Theme.Midnight, EffectLevel.Low, true, false, true),
            SettingsCodec.Sanitize(json));
    }

    [Fact]
    public void Sanitize_ReplacesInvalidFieldsWithTheirDefaultsOneByOne()
    {
        var result = SettingsCodec.Sanitize("""
            {"timeZone":"Atlantis/Lost_City","showMilliseconds":"yes","theme":"neon","effects":"ultra",
             "notify":1,"networkTime":false,"alwaysOnTop":null}
            """);
        Assert.Equal(SettingsData.Defaults with { NetworkTime = false }, result);
    }

    [Fact]
    public void Sanitize_IgnoresUnknownKeys()
    {
        var result = SettingsCodec.Sanitize("""{"theme":"dawn","injected":"<script>"}""");
        Assert.Equal(SettingsData.Defaults with { Theme = Theme.Dawn }, result);
        Assert.DoesNotContain("injected", SettingsCodec.Serialize(result));
    }

    [Fact]
    public void Sanitize_TreatsANullTimeZoneAsFollowingTheDevice() =>
        Assert.Null(SettingsCodec.Sanitize("""{"timeZone":null}""").TimeZone);

    [Fact]
    public void Serialize_WritesLowerCaseEnumNames()
    {
        var json = SettingsCodec.Serialize(SettingsData.Defaults);
        Assert.Contains("\"theme\":\"dusk\"", json);
        Assert.Contains("\"effects\":\"auto\"", json);
        Assert.Contains("\"timeZone\":null", json);
    }

    [Fact]
    public void Next_CyclesThroughTheThemes()
    {
        Assert.Equal(Theme.Midnight, SettingsCodec.Next(Theme.Dusk));
        Assert.Equal(Theme.Dawn, SettingsCodec.Next(Theme.Midnight));
        Assert.Equal(Theme.Dusk, SettingsCodec.Next(Theme.Dawn));
    }
}
