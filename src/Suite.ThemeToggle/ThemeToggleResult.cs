using Suite.Contracts;

namespace Suite.ThemeToggle;

public sealed class ThemeToggleResult
{
    public bool Succeeded { get; init; }
    public bool BroadcastSucceeded { get; init; }
    public ThemeKind NewTheme { get; init; }
    public string? Error { get; init; }

    public static ThemeToggleResult Fail(string error, ThemeKind theme) => new()
    {
        Succeeded = false,
        BroadcastSucceeded = false,
        NewTheme = theme,
        Error = error,
    };
}
