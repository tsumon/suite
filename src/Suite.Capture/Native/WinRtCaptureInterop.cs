using System.Runtime.InteropServices;

namespace Suite.Capture.Native;

[ComImport]
[Guid("3628E81B-3CAC-4C60-B7F4-23CE0E0C3356")]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IGraphicsCaptureItemInterop
{
    [PreserveSig]
    int CreateForWindow(IntPtr window, ref Guid iid, out IntPtr result);

    [PreserveSig]
    int CreateForMonitor(IntPtr monitor, ref Guid iid, out IntPtr result);
}

[ComImport]
[Guid("A9B3D012-3DF2-4EE3-B8D1-8695F457D3C1")]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IDirect3DDxgiInterfaceAccess
{
    [PreserveSig]
    int GetInterface(ref Guid iid, out IntPtr result);
}
