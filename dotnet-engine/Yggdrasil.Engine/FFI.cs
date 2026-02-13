using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Yggdrasil;


internal static class FFI
{
    private static readonly IntPtr _libHandle;

    static FFI()
    {
        _libHandle = NativeLibLoader.LoadNativeLibrary();

        // standard ygg messages, these require a buffer and return one
        check_enabled = LoadMsgCall("flat_check_enabled");
        check_variant = LoadMsgCall("flat_check_variant");
        define_counter = LoadMsgCall("flat_define_counter");
        inc_counter = LoadMsgCall("flat_inc_counter");
        define_gauge = LoadMsgCall("flat_define_gauge");
        set_gauge = LoadMsgCall("flat_set_gauge");
        define_histogram = LoadMsgCall("flat_define_histogram");
        observe_histogram = LoadMsgCall("flat_observe_histogram");

        new_engine = Marshal.GetDelegateForFunctionPointer<NewEngineDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "new_engine"));
        free_engine = Marshal.GetDelegateForFunctionPointer<FreeEngineDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "free_engine"));
        get_state = Marshal.GetDelegateForFunctionPointer<GetStateDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "get_state"));
        free_response = Marshal.GetDelegateForFunctionPointer<FreeResponseDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "free_response"));

        // everything else has some special details to it
        // if these get out of hand they'll need some refactor but for now 5 is fine
        take_state = Marshal.GetDelegateForFunctionPointer<TakeStateDelegate>(
            NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_take_state"));

        free_buffer = Marshal.GetDelegateForFunctionPointer<FreeBufferDelegate>(
            NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_buf_free"));

        list_known_toggles = Marshal.GetDelegateForFunctionPointer<ListKnownTogglesDelegate>(
            NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_list_known_toggles"));

        built_in_strategies = Marshal.GetDelegateForFunctionPointer<BuiltInStrategiesDelegate>(
            NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_built_in_strategies"));

        get_metrics = Marshal.GetDelegateForFunctionPointer<GetMetricsDelegate>(
            NativeLibLoader.LoadFunctionPointer(_libHandle, "flat_get_metrics"));
    }

    // one delegate type to rule them all, lets us not have to deal with a ton of delegate types in the higher layers
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate Buf MsgCallDelegate(IntPtr enginePtr, IntPtr messagePtr, nuint messageLen);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate Buf TakeStateDelegate(IntPtr ptr, byte[] json);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void FreeBufferDelegate(Buf buf);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate Buf ListKnownTogglesDelegate(IntPtr enginePtr);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate Buf BuiltInStrategiesDelegate();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate Buf GetMetricsDelegate(IntPtr enginePtr);
    private delegate IntPtr NewEngineDelegate();
    private delegate void FreeEngineDelegate(IntPtr ptr);
    private delegate IntPtr GetStateDelegate(IntPtr ptr);
    private delegate void FreeResponseDelegate(IntPtr ptr);
    private static readonly NewEngineDelegate new_engine;
    private static readonly FreeEngineDelegate free_engine;
    private static readonly GetStateDelegate get_state;
    private static readonly FreeResponseDelegate free_response;

    private static readonly TakeStateDelegate take_state;
    private static readonly FreeBufferDelegate free_buffer;

    private static readonly MsgCallDelegate check_enabled;
    private static readonly MsgCallDelegate check_variant;

    private static readonly MsgCallDelegate define_counter;
    private static readonly MsgCallDelegate inc_counter;
    private static readonly MsgCallDelegate define_gauge;
    private static readonly MsgCallDelegate set_gauge;
    private static readonly MsgCallDelegate define_histogram;
    private static readonly MsgCallDelegate observe_histogram;

    private static readonly ListKnownTogglesDelegate list_known_toggles;
    private static readonly BuiltInStrategiesDelegate built_in_strategies;
    private static readonly GetMetricsDelegate get_metrics;

    public static Buf TakeState(IntPtr ptr, string json)
        => take_state(ptr, ToUtf8NullTerminated(json));

    public static Buf CheckEnabled(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, check_enabled);

    public static Buf CheckVariant(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, check_variant);

    public static Buf DefineCounter(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, define_counter);

    public static Buf IncCounter(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, inc_counter);

    public static Buf DefineGauge(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, define_gauge);

    public static Buf SetGauge(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, set_gauge);

    public static Buf DefineHistogram(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, define_histogram);

    public static Buf ObserveHistogram(IntPtr ptr, byte[] message)
        => CallWithPinnedBytes(message, ptr, observe_histogram);

    public static Buf ListKnownToggles(IntPtr ptr) => list_known_toggles(ptr);
    public static Buf BuiltInStrategies() => built_in_strategies();
    public static Buf GetMetrics(IntPtr ptr) => get_metrics(ptr);

    public static void FreeBuf(Buf buf) => free_buffer(buf);


    private static MsgCallDelegate LoadMsgCall(string symbol)
        => Marshal.GetDelegateForFunctionPointer<MsgCallDelegate>(
            NativeLibLoader.LoadFunctionPointer(_libHandle, symbol));

    private static Buf CallWithPinnedBytes(byte[] message, IntPtr enginePtr, MsgCallDelegate nativeCall)
    {
        if (message is null) throw new ArgumentNullException(nameof(message));
        if (message.Length == 0) return nativeCall(enginePtr, IntPtr.Zero, 0);

        var len = (nuint)message.Length;
        var handle = GCHandle.Alloc(message, GCHandleType.Pinned);
        try
        {
            return nativeCall(enginePtr, handle.AddrOfPinnedObject(), len);
        }
        finally
        {
            handle.Free();
        }
    }

    private static byte[] ToUtf8NullTerminated(string input)
    {
        if (input is null) throw new ArgumentNullException(nameof(input));

        byte[] utf8Bytes = Encoding.UTF8.GetBytes(input);
        Array.Resize(ref utf8Bytes, utf8Bytes.Length + 1);
        return utf8Bytes;
    }

    public static IntPtr NewEngine()
    {
        return new_engine();
    }

    public static void FreeEngine(IntPtr ptr)
    {
        free_engine(ptr);
    }

    public static IntPtr GetState(IntPtr ptr)
    {
        return get_state(ptr);
    }

    public static void FreeResponse(IntPtr ptr)
    {
        free_response(ptr);
    }
}


// internal static class FFI
// {
//     private static IntPtr _libHandle;

//     static FFI()
//     {
//         _libHandle = NativeLibLoader.LoadNativeLibrary();

//         new_engine = Marshal.GetDelegateForFunctionPointer<NewEngineDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "new_engine"));
//         free_engine = Marshal.GetDelegateForFunctionPointer<FreeEngineDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "free_engine"));
//         get_state = Marshal.GetDelegateForFunctionPointer<GetStateDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "get_state"));
//         free_response = Marshal.GetDelegateForFunctionPointer<FreeResponseDelegate>(NativeLibLoader.LoadFunctionPointer(_libHandle, "free_response"));
//     }

//     private delegate IntPtr NewEngineDelegate();
//     private delegate void FreeEngineDelegate(IntPtr ptr);
//     private delegate IntPtr GetStateDelegate(IntPtr ptr);
//     private delegate void FreeResponseDelegate(IntPtr ptr);
//     private static readonly NewEngineDelegate new_engine;
//     private static readonly FreeEngineDelegate free_engine;
//     private static readonly GetStateDelegate get_state;
//     private static readonly FreeResponseDelegate free_response;

//     public static IntPtr NewEngine()
//     {
//         return new_engine();
//     }

//     public static void FreeEngine(IntPtr ptr)
//     {
//         free_engine(ptr);
//     }

//     public static IntPtr GetState(IntPtr ptr)
//     {
//         return get_state(ptr);
//     }

//     public static void FreeResponse(IntPtr ptr)
//     {
//         free_response(ptr);
//     }
// }
