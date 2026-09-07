namespace Suite.Capture.Native;

internal static unsafe class ComVtable
{
    public static int QueryInterface(IntPtr obj, Guid iid, out IntPtr ppv)
    {
        ppv = IntPtr.Zero;
        if (obj == IntPtr.Zero)
        {
            return unchecked((int)0x80004003);
        }

        var vt = *(IntPtr**)obj;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, Guid*, IntPtr*, int>)vt[0];
        Guid g = iid;
        IntPtr result;
        int hr = fn(obj, &g, &result);
        ppv = result;
        return hr;
    }

    public static uint AddRef(IntPtr obj)
    {
        var vt = *(IntPtr**)obj;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, uint>)vt[1];
        return fn(obj);
    }

    public static uint Release(IntPtr obj)
    {
        if (obj == IntPtr.Zero)
        {
            return 0;
        }

        var vt = *(IntPtr**)obj;
        var fn = (delegate* unmanaged[Stdcall]<IntPtr, uint>)vt[2];
        return fn(obj);
    }

    public static void ReleaseQuiet(ref IntPtr obj)
    {
        if (obj == IntPtr.Zero)
        {
            return;
        }

        Release(obj);
        obj = IntPtr.Zero;
    }
}

internal sealed class ComPtr : IDisposable
{
    private IntPtr _ptr;

    public ComPtr(IntPtr ptr) => _ptr = ptr;

    public IntPtr Ptr => _ptr;

    public bool IsNull => _ptr == IntPtr.Zero;

    public void Dispose()
    {
        ComVtable.ReleaseQuiet(ref _ptr);
    }
}
