using Suite.Contracts;
using Suite.ThemeToggle;

namespace Suite.ThemeToggle.Tests;

internal sealed class FakePersonalizeStore : IPersonalizeStore
{
    public ThemeKind Apps { get; set; } = ThemeKind.Light;
    public ThemeKind System { get; set; } = ThemeKind.Light;
    public bool ReadShouldFail { get; set; }
    public bool WriteShouldFail { get; set; }
    public int WriteCount { get; private set; }
    public ThemeKind? LastWrittenApps { get; private set; }
    public ThemeKind? LastWrittenSystem { get; private set; }

    public bool TryRead(out ThemeKind apps, out ThemeKind system, out string? error)
    {
        apps = Apps;
        system = System;
        if (ReadShouldFail)
        {
            error = "read denied";
            return false;
        }

        error = null;
        return true;
    }

    public bool TryWrite(ThemeKind apps, ThemeKind system, out string? error)
    {
        WriteCount++;
        LastWrittenApps = apps;
        LastWrittenSystem = system;
        if (WriteShouldFail)
        {
            error = "policy locked";
            return false;
        }

        Apps = apps;
        System = system;
        error = null;
        return true;
    }
}
