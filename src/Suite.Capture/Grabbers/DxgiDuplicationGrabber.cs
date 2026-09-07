using Suite.Capture.Native;

namespace Suite.Capture.Grabbers;

internal static unsafe class DxgiDuplicationGrabber
{
    public static bool TryCapture(MonitorInfo monitor, out PixelBuffer? buffer)
    {
        buffer = null;
        IntPtr factory = IntPtr.Zero;
        try
        {
            Guid factoryIid = CaptureNative.IidDxgiFactory1;
            if (CaptureNative.CreateDXGIFactory1(ref factoryIid, out factory) < 0 || factory == IntPtr.Zero)
            {
                return false;
            }

            for (uint adapterIndex = 0; ; adapterIndex++)
            {
                IntPtr adapter = IntPtr.Zero;
                int hr = EnumAdapters1(factory, adapterIndex, out adapter);
                if (hr == CaptureNative.DxgiErrorNotFound)
                {
                    break;
                }

                if (hr < 0 || adapter == IntPtr.Zero)
                {
                    continue;
                }

                try
                {
                    if (TryCaptureOnAdapter(adapter, monitor, out buffer))
                    {
                        return true;
                    }
                }
                finally
                {
                    ComVtable.ReleaseQuiet(ref adapter);
                }
            }

            return false;
        }
        catch
        {
            return false;
        }
        finally
        {
            ComVtable.ReleaseQuiet(ref factory);
        }
    }

    private static bool TryCaptureOnAdapter(IntPtr adapter, MonitorInfo monitor, out PixelBuffer? buffer)
    {
        buffer = null;
        for (uint outputIndex = 0; ; outputIndex++)
        {
            IntPtr output = IntPtr.Zero;
            int hr = EnumOutputs(adapter, outputIndex, out output);
            if (hr == CaptureNative.DxgiErrorNotFound)
            {
                return false;
            }

            if (hr < 0 || output == IntPtr.Zero)
            {
                continue;
            }

            IntPtr output1 = IntPtr.Zero;
            IntPtr device = IntPtr.Zero;
            IntPtr context = IntPtr.Zero;
            IntPtr duplication = IntPtr.Zero;
            IntPtr resource = IntPtr.Zero;
            IntPtr texture = IntPtr.Zero;
            try
            {
                var desc = new DxgiOutputDesc();
                if (GetOutputDesc(output, ref desc) < 0)
                {
                    continue;
                }

                if (monitor.Handle != IntPtr.Zero && desc.Monitor != monitor.Handle)
                {
                    continue;
                }

                if (monitor.Handle == IntPtr.Zero)
                {
                    var outputBounds = new PixelRect(
                        desc.DesktopCoordinates.Left,
                        desc.DesktopCoordinates.Top,
                        desc.DesktopCoordinates.Right - desc.DesktopCoordinates.Left,
                        desc.DesktopCoordinates.Bottom - desc.DesktopCoordinates.Top);
                    if (!outputBounds.Equals(monitor.Bounds))
                    {
                        continue;
                    }
                }

                if (ComVtable.QueryInterface(output, CaptureNative.IidDxgiOutput1, out output1) < 0
                    || output1 == IntPtr.Zero)
                {
                    return false;
                }

                if (!D3dDevice.TryCreate(adapter, out device, out context))
                {
                    return false;
                }

                hr = DuplicateOutput(output1, device, out duplication);
                if (hr < 0 || duplication == IntPtr.Zero)
                {
                    return false;
                }

                hr = AcquireNextFrame(duplication, 800, out resource);
                if (hr == CaptureNative.DxgiErrorWaitTimeout)
                {
                    hr = AcquireNextFrame(duplication, 800, out resource);
                }

                if (hr < 0 || resource == IntPtr.Zero)
                {
                    return false;
                }

                if (ComVtable.QueryInterface(resource, CaptureNative.IidD3d11Texture2D, out texture) < 0
                    || texture == IntPtr.Zero)
                {
                    return false;
                }

                return D3dDevice.TryCopyTexture(device, context, texture, out buffer);
            }
            catch
            {
                return false;
            }
            finally
            {
                if (duplication != IntPtr.Zero && resource != IntPtr.Zero)
                {
                    ReleaseFrame(duplication);
                }

                ComVtable.ReleaseQuiet(ref texture);
                ComVtable.ReleaseQuiet(ref resource);
                ComVtable.ReleaseQuiet(ref duplication);
                ComVtable.ReleaseQuiet(ref context);
                ComVtable.ReleaseQuiet(ref device);
                ComVtable.ReleaseQuiet(ref output1);
                ComVtable.ReleaseQuiet(ref output);
            }
        }
    }

    private static int EnumAdapters1(IntPtr factory, uint index, out IntPtr adapter)
    {
        var vt = *(IntPtr**)factory;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, uint, IntPtr*, int>)vt[12];
        IntPtr result;
        int hr = fn(factory, index, &result);
        adapter = result;
        return hr;
    }

    private static int EnumOutputs(IntPtr adapter, uint index, out IntPtr output)
    {
        var vt = *(IntPtr**)adapter;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, uint, IntPtr*, int>)vt[7];
        IntPtr result;
        int hr = fn(adapter, index, &result);
        output = result;
        return hr;
    }

    private static int GetOutputDesc(IntPtr output, ref DxgiOutputDesc desc)
    {
        var vt = *(IntPtr**)output;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, DxgiOutputDesc*, int>)vt[7];
        fixed (DxgiOutputDesc* p = &desc)
        {
            return fn(output, p);
        }
    }

    private static int DuplicateOutput(IntPtr output1, IntPtr device, out IntPtr duplication)
    {
        var vt = *(IntPtr**)output1;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, IntPtr, IntPtr*, int>)vt[22];
        IntPtr result;
        int hr = fn(output1, device, &result);
        duplication = result;
        return hr;
    }

    private static int AcquireNextFrame(IntPtr duplication, uint timeoutMs, out IntPtr resource)
    {
        var vt = *(IntPtr**)duplication;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, uint, byte*, IntPtr*, int>)vt[8];
        var info = new byte[128];
        IntPtr result;
        int hr;
        fixed (byte* pInfo = info)
        {
            hr = fn(duplication, timeoutMs, pInfo, &result);
        }

        resource = result;
        return hr;
    }

    private static int ReleaseFrame(IntPtr duplication)
    {
        var vt = *(IntPtr**)duplication;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, int>)vt[14];
        return fn(duplication);
    }
}
