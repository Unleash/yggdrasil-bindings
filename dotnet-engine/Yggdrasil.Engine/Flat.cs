using System;
using System.Runtime.InteropServices;
namespace Yggdrasil;

internal static class Flat
{
    private static IntPtr _libHandle;

    static Flat()
    {
        _libHandle = NativeLibLoader.LoadNativeLibrary();
        take_state = Marshal.GetDelegateForFunctionPointer<TakeStateDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_take_state"));
        free_buffer = Marshal.GetDelegateForFunctionPointer<FreeBufferDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_buf_free"));
        check_enabled = Marshal.GetDelegateForFunctionPointer<CheckEnabledDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_check_enabled"));
        check_variant = Marshal.GetDelegateForFunctionPointer<CheckVariantDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_check_variant"));
        list_known_toggles = Marshal.GetDelegateForFunctionPointer<ListKnownTogglesDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_list_known_toggles"));
        built_in_strategies = Marshal.GetDelegateForFunctionPointer<BuiltInStrategiesDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_built_in_strategies"));
        get_metrics = Marshal.GetDelegateForFunctionPointer<GetMetricsDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_get_metrics"));
    }

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate Buf TakeStateDelegate(IntPtr ptr, byte[] json);

    private delegate void FreeBufferDelegate(Buf buf);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate Buf CheckEnabledDelegate(IntPtr enginePtr, IntPtr messagePtr, nuint messageLen);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate Buf CheckVariantDelegate(IntPtr enginePtr, IntPtr messagePtr, nuint messageLen);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate Buf ListKnownTogglesDelegate(IntPtr enginePtr);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate Buf BuiltInStrategiesDelegate(IntPtr enginePtr);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate Buf GetMetricsDelegate(IntPtr enginePtr);
    private static readonly TakeStateDelegate take_state;
    private static readonly FreeBufferDelegate free_buffer;
    private static readonly CheckEnabledDelegate check_enabled;
    private static readonly CheckVariantDelegate check_variant;
    private static readonly ListKnownTogglesDelegate list_known_toggles;
    private static readonly BuiltInStrategiesDelegate built_in_strategies;
    private static readonly GetMetricsDelegate get_metrics;

    public static Buf TakeState(IntPtr ptr, string json)
    {
        return take_state(ptr, ToUtf8Bytes(json));
    }

    public static Buf CheckEnabled(IntPtr ptr, byte[] message)
    {
        nuint len = (nuint)message.Length;
        GCHandle handle = GCHandle.Alloc(message, GCHandleType.Pinned);
        try
        {
            IntPtr msgPtr = handle.AddrOfPinnedObject();
            return check_enabled(ptr, msgPtr, len);
        }
        finally
        {
            handle.Free();
        }
    }

    public static Buf CheckVariant(IntPtr ptr, byte[] message)
    {
        nuint len = (nuint)message.Length;
        GCHandle handle = GCHandle.Alloc(message, GCHandleType.Pinned);
        try
        {
            IntPtr msgPtr = handle.AddrOfPinnedObject();
            return check_variant(ptr, msgPtr, len);
        }
        finally
        {
            handle.Free();
        }
    }
    public static Buf ListKnownToggles(IntPtr ptr)
    {
        return list_known_toggles(ptr);
    }

    public static Buf BuiltInStrategies(IntPtr ptr)
    {
        return built_in_strategies(ptr);
    }

    public static Buf GetMetrics(IntPtr ptr)
    {
        return get_metrics(ptr);
    }

    public static void FreeBuf(Buf buf)
    {
        free_buffer(buf);
    }

    private static byte[] ToUtf8Bytes(string input)
    {
        byte[] utf8Bytes = System.Text.Encoding.UTF8.GetBytes(input);
        Array.Resize(ref utf8Bytes, utf8Bytes.Length + 1);
        return utf8Bytes;
    }
}
