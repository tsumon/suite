using System.Runtime.InteropServices;

namespace Suite.Capture.Native;

internal static unsafe class D3dDevice
{
    public static bool TryCreate(IntPtr adapter, out IntPtr device, out IntPtr context)
    {
        device = IntPtr.Zero;
        context = IntPtr.Zero;
        int hr = CaptureNative.D3D11CreateDevice(
            adapter,
            adapter == IntPtr.Zero ? CaptureNative.D3dDriverTypeHardware : 0,
            IntPtr.Zero,
            CaptureNative.D3d11CreateDeviceBgraSupport,
            IntPtr.Zero,
            0,
            CaptureNative.D3d11SdkVersion,
            out device,
            out _,
            out context);
        if (hr >= 0 && device != IntPtr.Zero && context != IntPtr.Zero)
        {
            return true;
        }

        ComVtable.ReleaseQuiet(ref device);
        ComVtable.ReleaseQuiet(ref context);
        if (adapter != IntPtr.Zero)
        {
            return false;
        }

        hr = CaptureNative.D3D11CreateDevice(
            IntPtr.Zero,
            CaptureNative.D3dDriverTypeWarp,
            IntPtr.Zero,
            CaptureNative.D3d11CreateDeviceBgraSupport,
            IntPtr.Zero,
            0,
            CaptureNative.D3d11SdkVersion,
            out device,
            out _,
            out context);
        if (hr >= 0 && device != IntPtr.Zero && context != IntPtr.Zero)
        {
            return true;
        }

        ComVtable.ReleaseQuiet(ref device);
        ComVtable.ReleaseQuiet(ref context);
        return false;
    }

    public static bool TryCopyTexture(IntPtr device, IntPtr context, IntPtr texture, out PixelBuffer? buffer)
    {
        buffer = null;
        IntPtr staging = IntPtr.Zero;
        try
        {
            var desc = new D3d11Texture2dDesc();
            GetTextureDesc(texture, ref desc);
            if (desc.Width == 0 || desc.Height == 0)
            {
                return false;
            }

            desc.MipLevels = 1;
            desc.ArraySize = 1;
            desc.SampleCount = 1;
            desc.SampleQuality = 0;
            desc.Usage = CaptureNative.D3d11UsageStaging;
            desc.BindFlags = 0;
            desc.CpuAccessFlags = CaptureNative.D3d11CpuAccessRead;
            desc.MiscFlags = 0;
            if (desc.Format == 0)
            {
                desc.Format = CaptureNative.DxgiFormatB8G8R8A8Unorm;
            }

            int hr = CreateTexture2D(device, ref desc, out staging);
            if (hr < 0 || staging == IntPtr.Zero)
            {
                return false;
            }

            CopyResource(context, staging, texture);
            var mapped = new D3d11MappedSubresource();
            hr = Map(context, staging, ref mapped);
            if (hr < 0 || mapped.PData == IntPtr.Zero)
            {
                return false;
            }

            try
            {
                int width = (int)desc.Width;
                int height = (int)desc.Height;
                int stride = width * 4;
                var pixels = new byte[stride * height];
                int rowPitch = (int)mapped.RowPitch;
                for (int y = 0; y < height; y++)
                {
                    IntPtr src = mapped.PData + (y * rowPitch);
                    Marshal.Copy(src, pixels, y * stride, stride);
                }

                buffer = new PixelBuffer(width, height, pixels, stride);
                return true;
            }
            finally
            {
                Unmap(context, staging);
            }
        }
        catch
        {
            return false;
        }
        finally
        {
            ComVtable.ReleaseQuiet(ref staging);
        }
    }

    private static void GetTextureDesc(IntPtr texture, ref D3d11Texture2dDesc desc)
    {
        var vt = *(IntPtr**)texture;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, D3d11Texture2dDesc*, void>)vt[10];
        fixed (D3d11Texture2dDesc* p = &desc)
        {
            fn(texture, p);
        }
    }

    private static int CreateTexture2D(IntPtr device, ref D3d11Texture2dDesc desc, out IntPtr texture)
    {
        var vt = *(IntPtr**)device;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, D3d11Texture2dDesc*, IntPtr, IntPtr*, int>)vt[5];
        IntPtr result;
        int hr;
        fixed (D3d11Texture2dDesc* p = &desc)
        {
            hr = fn(device, p, IntPtr.Zero, &result);
        }

        texture = result;
        return hr;
    }

    private static void CopyResource(IntPtr context, IntPtr dest, IntPtr src)
    {
        var vt = *(IntPtr**)context;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, IntPtr, IntPtr, void>)vt[47];
        fn(context, dest, src);
    }

    private static int Map(IntPtr context, IntPtr resource, ref D3d11MappedSubresource mapped)
    {
        var vt = *(IntPtr**)context;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, IntPtr, uint, uint, uint, D3d11MappedSubresource*, int>)vt[14];
        fixed (D3d11MappedSubresource* p = &mapped)
        {
            return fn(context, resource, 0, CaptureNative.D3d11MapRead, 0, p);
        }
    }

    private static void Unmap(IntPtr context, IntPtr resource)
    {
        var vt = *(IntPtr**)context;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, IntPtr, uint, void>)vt[15];
        fn(context, resource, 0);
    }
}
