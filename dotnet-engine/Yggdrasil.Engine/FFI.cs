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
    private delegate Buf FlatBufferMessageDelegate(IntPtr enginePtr, byte[] message, nuint messageLen);

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

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate IntPtr NewEngineDelegate();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void FreeEngineDelegate(IntPtr ptr);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate IntPtr GetStateDelegate(IntPtr ptr);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void FreeResponseDelegate(IntPtr ptr);
    private static readonly NewEngineDelegate new_engine;
    private static readonly FreeEngineDelegate free_engine;
    private static readonly GetStateDelegate get_state;
    private static readonly FreeResponseDelegate free_response;

    private static readonly TakeStateDelegate take_state;
    private static readonly FreeBufferDelegate free_buffer;

    private static readonly FlatBufferMessageDelegate check_enabled;
    private static readonly FlatBufferMessageDelegate check_variant;

    private static readonly FlatBufferMessageDelegate define_counter;
    private static readonly FlatBufferMessageDelegate inc_counter;
    private static readonly FlatBufferMessageDelegate define_gauge;
    private static readonly FlatBufferMessageDelegate set_gauge;
    private static readonly FlatBufferMessageDelegate define_histogram;
    private static readonly FlatBufferMessageDelegate observe_histogram;

    private static readonly ListKnownTogglesDelegate list_known_toggles;
    private static readonly BuiltInStrategiesDelegate built_in_strategies;
    private static readonly GetMetricsDelegate get_metrics;

    internal static Buf TakeState(IntPtr ptr, string json)
        => take_state(ptr, ToUtf8NullTerminated(json));

    internal static Buf CheckEnabled(IntPtr ptr, byte[] message)
        => check_enabled(ptr, message, (nuint)message.Length);

    internal static Buf CheckVariant(IntPtr ptr, byte[] message)
        => Call(message, ptr, check_variant);

    internal static Buf DefineCounter(IntPtr ptr, byte[] message)
        => Call(message, ptr, define_counter);

    internal static Buf IncCounter(IntPtr ptr, byte[] message)
        => Call(message, ptr, inc_counter);

    internal static Buf DefineGauge(IntPtr ptr, byte[] message)
        => Call(message, ptr, define_gauge);

    internal static Buf SetGauge(IntPtr ptr, byte[] message)
        => Call(message, ptr, set_gauge);

    internal static Buf DefineHistogram(IntPtr ptr, byte[] message)
        => Call(message, ptr, define_histogram);

    internal static Buf ObserveHistogram(IntPtr ptr, byte[] message)
        => Call(message, ptr, observe_histogram);

    internal static Buf ListKnownToggles(IntPtr ptr) => list_known_toggles(ptr);

    internal static Buf BuiltInStrategies() => built_in_strategies();

    internal static Buf GetMetrics(IntPtr ptr) => get_metrics(ptr);

    internal static void FreeBuf(Buf buf) => free_buffer(buf);

    internal static IntPtr NewEngine()
    {
        return new_engine();
    }

    internal static void FreeEngine(IntPtr ptr)
    {
        free_engine(ptr);
    }

    internal static IntPtr GetState(IntPtr ptr)
    {
        return get_state(ptr);
    }

    internal static void FreeResponse(IntPtr ptr)
    {
        free_response(ptr);
    }

    private static FlatBufferMessageDelegate LoadMsgCall(string symbol)
    => Marshal.GetDelegateForFunctionPointer<FlatBufferMessageDelegate>(
        NativeLibLoader.LoadFunctionPointer(_libHandle, symbol));

    private static Buf Call(byte[] message, IntPtr enginePtr, FlatBufferMessageDelegate nativeCall)
    {
        if (message is null) throw new ArgumentNullException(nameof(message));

        return nativeCall(enginePtr, message, (nuint)message.Length);
    }

    private static byte[] ToUtf8NullTerminated(string input)
    {
        if (input is null) throw new ArgumentNullException(nameof(input));

        byte[] utf8Bytes = Encoding.UTF8.GetBytes(input);
        Array.Resize(ref utf8Bytes, utf8Bytes.Length + 1);
        return utf8Bytes;
    }
}