using System.Security.Principal;

namespace Suite.Platform;

public static class ProcessIntegrity
{
    public static bool IsElevatedAdministrator()
    {
        using var identity = WindowsIdentity.GetCurrent();
        var principal = new WindowsPrincipal(identity);
        return principal.IsInRole(WindowsBuiltInRole.Administrator);
    }
}
