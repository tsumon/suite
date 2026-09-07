namespace Suite.Contracts;

/// <summary>
/// v1 appearance rungs. Blur and dynamic modes are out of P1.
/// Wire values are lowercase to match the IPC protocol.
/// </summary>
public enum TaskbarAppearanceMode
{
    Normal = 0,
    Opaque = 1,
    Clear = 2,
    Acrylic = 3,
}

public static class TaskbarAppearanceModes
{
    public const string Normal = "normal";
    public const string Opaque = "opaque";
    public const string Clear = "clear";
    public const string Acrylic = "acrylic";

    public static string ToWire(TaskbarAppearanceMode mode) => mode switch
    {
        TaskbarAppearanceMode.Opaque => Opaque,
        TaskbarAppearanceMode.Clear => Clear,
        TaskbarAppearanceMode.Acrylic => Acrylic,
        _ => Normal,
    };

    public static bool TryParse(string? value, out TaskbarAppearanceMode mode)
    {
        switch ((value ?? "").Trim().ToLowerInvariant())
        {
            case Opaque:
                mode = TaskbarAppearanceMode.Opaque;
                return true;
            case Clear:
                mode = TaskbarAppearanceMode.Clear;
                return true;
            case Acrylic:
                mode = TaskbarAppearanceMode.Acrylic;
                return true;
            case Normal:
            case "":
                mode = TaskbarAppearanceMode.Normal;
                return true;
            default:
                mode = TaskbarAppearanceMode.Normal;
                return false;
        }
    }
}
