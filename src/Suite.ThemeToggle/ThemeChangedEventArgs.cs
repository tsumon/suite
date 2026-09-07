using Suite.Contracts;

namespace Suite.ThemeToggle;

public sealed class ThemeChangedEventArgs : EventArgs
{
    public ThemeChangedEventArgs(ThemeKind theme) => Theme = theme;

    public ThemeKind Theme { get; }
}
