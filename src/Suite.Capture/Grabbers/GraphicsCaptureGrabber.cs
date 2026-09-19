using System.Runtime.InteropServices;
using Suite.Capture.Native;
using Windows.Foundation;
using Windows.Graphics.Capture;
using Windows.Graphics.DirectX;
using Windows.Graphics.DirectX.Direct3D11;
using WinRT;

namespace Suite.Capture.Grabbers;

internal static class GraphicsCaptureGrabber
{
    public static bool TryCapture(MonitorInfo monitor, out PixelBuffer? buffer)
    {
        buffer = null;
        if (monitor.Handle == IntPtr.Zero)
        {
            return false;
        }

        IntPtr device = IntPtr.Zero;
        IntPtr context = IntPtr.Zero;
        IntPtr dxgiDevice = IntPtr.Zero;
        IntPtr winrtDeviceAbi = IntPtr.Zero;
        Direct3D11CaptureFramePool? pool = null;
        GraphicsCaptureSession? session = null;
        Direct3D11CaptureFrame? frame = null;
        ManualResetEventSlim? arrived = null;
        TypedEventHandler<Direct3D11CaptureFramePool, object>? frameArrived = null;
        int callbackClosed = 0;
        try
        {
            if (!GraphicsCaptureSession.IsSupported())
            {
                return false;
            }

            if (!TryCreateItem(monitor.Handle, out GraphicsCaptureItem? item) || item is null)
            {
                return false;
            }

            if (!D3dDevice.TryCreate(IntPtr.Zero, out device, out context))
            {
                return false;
            }

            if (ComVtable.QueryInterface(device, CaptureNative.IidDxgiDevice, out dxgiDevice) < 0
                || dxgiDevice == IntPtr.Zero)
            {
                return false;
            }

            if (CaptureNative.CreateDirect3D11DeviceFromDXGIDevice(dxgiDevice, out winrtDeviceAbi) < 0
                || winrtDeviceAbi == IntPtr.Zero)
            {
                return false;
            }

            IDirect3DDevice winrtDevice = MarshalInterface<IDirect3DDevice>.FromAbi(winrtDeviceAbi);
            pool = Direct3D11CaptureFramePool.CreateFreeThreaded(
                winrtDevice,
                DirectXPixelFormat.B8G8R8A8UIntNormalized,
                1,
                item.Size);
            if (pool is null)
            {
                return false;
            }

            session = pool.CreateCaptureSession(item);
            if (session is null)
            {
                return false;
            }

            try
            {
                session.IsCursorCaptureEnabled = false;
            }
            catch (Exception)
            {
            }

            // Win11 yellow border: set IsBorderRequired=false via reflection when present.
            // If property missing or GraphicsCapture fails, ScreenCapturePipeline falls back to DXGI then GDI (no yellow border).
            try
            {
                System.Reflection.PropertyInfo? border = session.GetType().GetProperty("IsBorderRequired");
                if (border is not null && border.CanWrite)
                {
                    border.SetValue(session, false);
                }
            }
            catch (Exception)
            {
            }

            arrived = new ManualResetEventSlim(false);
            Direct3D11CaptureFrame? captured = null;
            Direct3D11CaptureFramePool framePool = pool;
            frameArrived = (_, _) =>
            {
                if (Volatile.Read(ref callbackClosed) != 0)
                {
                    return;
                }

                if (captured is null)
                {
                    captured = framePool.TryGetNextFrame();
                }

                arrived.Set();
            };
            framePool.FrameArrived += frameArrived;
            session.StartCapture();
            arrived.Wait(TimeSpan.FromMilliseconds(1500));
            if (captured is null)
            {
                captured = framePool.TryGetNextFrame();
            }

            frame = captured;
            if (frame is null)
            {
                return false;
            }

            if (!TryGetTexture(frame.Surface, out IntPtr texture) || texture == IntPtr.Zero)
            {
                return false;
            }

            try
            {
                return D3dDevice.TryCopyTexture(device, context, texture, out buffer);
            }
            finally
            {
                ComVtable.Release(texture);
            }
        }
        catch
        {
            return false;
        }
        finally
        {
            Volatile.Write(ref callbackClosed, 1);
            if (pool is not null && frameArrived is not null)
            {
                pool.FrameArrived -= frameArrived;
            }

            arrived?.Dispose();
            DisposeWinRt(frame);
            DisposeWinRt(session);
            DisposeWinRt(pool);
            ComVtable.ReleaseQuiet(ref winrtDeviceAbi);
            ComVtable.ReleaseQuiet(ref dxgiDevice);
            ComVtable.ReleaseQuiet(ref context);
            ComVtable.ReleaseQuiet(ref device);
        }
    }

    private static void DisposeWinRt(IDisposable? obj)
    {
        if (obj is not null)
        {
            obj.Dispose();
        }
    }

    private static bool TryCreateItem(IntPtr hmonitor, out GraphicsCaptureItem? item)
    {
        item = null;
        IntPtr hstring = IntPtr.Zero;
        IntPtr factory = IntPtr.Zero;
        IntPtr interop = IntPtr.Zero;
        IntPtr itemAbi = IntPtr.Zero;
        try
        {
            const string classId = "Windows.Graphics.Capture.GraphicsCaptureItem";
            if (CaptureNative.WindowsCreateString(classId, classId.Length, out hstring) < 0)
            {
                return false;
            }

            Guid factoryIid = CaptureNative.IidActivationFactory;
            if (CaptureNative.RoGetActivationFactory(hstring, ref factoryIid, out factory) < 0
                || factory == IntPtr.Zero)
            {
                return false;
            }

            if (ComVtable.QueryInterface(factory, CaptureNative.IidGraphicsCaptureItemInterop, out interop) < 0
                || interop == IntPtr.Zero)
            {
                return false;
            }

            var interopObj = (IGraphicsCaptureItemInterop)Marshal.GetObjectForIUnknown(interop);
            Guid itemIid = CaptureNative.IidGraphicsCaptureItem;
            int hr = interopObj.CreateForMonitor(hmonitor, ref itemIid, out itemAbi);
            if (hr < 0 || itemAbi == IntPtr.Zero)
            {
                return false;
            }

            item = MarshalInspectable<GraphicsCaptureItem>.FromAbi(itemAbi);
            return item is not null;
        }
        catch
        {
            return false;
        }
        finally
        {
            ComVtable.ReleaseQuiet(ref itemAbi);
            ComVtable.ReleaseQuiet(ref interop);
            ComVtable.ReleaseQuiet(ref factory);
            if (hstring != IntPtr.Zero)
            {
                CaptureNative.WindowsDeleteString(hstring);
            }
        }
    }

    private static bool TryGetTexture(IDirect3DSurface surface, out IntPtr texture)
    {
        texture = IntPtr.Zero;
        IntPtr inspectable = IntPtr.Zero;
        IntPtr access = IntPtr.Zero;
        try
        {
            inspectable = MarshalInterface<IDirect3DSurface>.FromManaged(surface);
            if (ComVtable.QueryInterface(inspectable, CaptureNative.IidDirect3DDxgiInterfaceAccess, out access) < 0
                || access == IntPtr.Zero)
            {
                return false;
            }

            var accessObj = (IDirect3DDxgiInterfaceAccess)Marshal.GetObjectForIUnknown(access);
            Guid textureIid = CaptureNative.IidD3d11Texture2D;
            return accessObj.GetInterface(ref textureIid, out texture) >= 0 && texture != IntPtr.Zero;
        }
        catch
        {
            return false;
        }
        finally
        {
            ComVtable.ReleaseQuiet(ref access);
            ComVtable.ReleaseQuiet(ref inspectable);
        }
    }
}
