using System.Runtime.InteropServices;
using System.Text.Json;
using Google.FlatBuffers;
using yggdrasil.messaging;

namespace Yggdrasil;

public sealed class YggdrasilEngine : IDisposable
{
    private const int INITIAL_BUFFER_SIZE = 256;

    [ThreadStatic]
    private static FlatBufferBuilder? t_builder;

    private static FlatBufferBuilder Builder
    {
        get
        {
            var b = t_builder;
            if (b is null)
            {
                b = new FlatBufferBuilder(INITIAL_BUFFER_SIZE);
                t_builder = b;
            }
            else
            {
                b.Clear();
            }

            return b;
        }
    }

    private delegate Buf NativeCall(IntPtr state, byte[] msg);

    private static readonly JsonSerializerOptions jsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    private readonly CustomStrategies customStrategies;
    private IntPtr state;
    private bool disposed;

    public YggdrasilEngine(List<IStrategy>? strategies = null)
    {
        state = FFI.NewEngine();

        var buf = FFI.BuiltInStrategies();
        try
        {
            var knownStrategies = Flatbuffers.GetBuiltInStrategiesResponse(buf);
            customStrategies = new CustomStrategies(knownStrategies);

            if (strategies is not null)
            {
                customStrategies.RegisterCustomStrategies(strategies);
            }
        }
        finally
        {
            FFI.FreeBuf(buf);
        }
    }

    ~YggdrasilEngine()
    {
        DisposeCore();
    }

    public void Dispose()
    {
        DisposeCore();
        GC.SuppressFinalize(this);
    }

    private void DisposeCore()
    {
        if (disposed) return;
        disposed = true;

        if (state != IntPtr.Zero)
        {
            FFI.FreeEngine(state);
            state = IntPtr.Zero;
        }
    }

    public void TakeState(string json)
    {
        EnsureNotDisposed();

        var buf = FFI.TakeState(state, json);
        try
        {
            var takeStateResponse = Flatbuffers.GetTakeStateResponse(buf);
            customStrategies.MapFeatures(takeStateResponse);
        }
        finally
        {
            FFI.FreeBuf(buf);
        }
    }

    public string GetState()
    {
        EnsureNotDisposed();

        var getStatePtr = FFI.GetState(state);
        var stateObject = ReadComplex<object>(getStatePtr);
        return JsonSerializer.Serialize(stateObject, jsonOptions);
    }

    public Response IsEnabled(string toggleName, Context context)
    {
        EnsureNotDisposed();

        var customStrategies = this.customStrategies.GetCustomStrategyResults(toggleName, context);
        var msg = Flatbuffers.GetContextMessageBuffer(Builder, toggleName, context, customStrategies);

        return Invoke(FFI.CheckEnabled, msg, Flatbuffers.GetCheckEnabledResponse);
    }

    public Variant? GetVariant(string toggleName, Context context)
    {
        EnsureNotDisposed();

        var customStrategies = this.customStrategies.GetCustomStrategyResults(toggleName, context);
        var msg = Flatbuffers.GetContextMessageBuffer(Builder, toggleName, context, customStrategies);

        return Invoke(FFI.CheckVariant, msg, Flatbuffers.GetCheckVariantResponse);
    }

    public MetricsBucket? GetMetrics()
    {
        EnsureNotDisposed();
        return InvokeNoMsg(FFI.GetMetrics, Flatbuffers.GetMetricsBucket);
    }

    public void DefineCounter(string name, string help)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateDefineCounterBuffer(Builder, name, help);
        InvokeVoid(FFI.DefineCounter, msg);
    }

    public void IncCounter(string name, long value = 1, IDictionary<string, string>? labels = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateIncCounterBuffer(Builder, name, value, labels);
        InvokeVoid(FFI.IncCounter, msg);
    }

    public void DefineGauge(string name, string help)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateDefineGaugeBuffer(Builder, name, help);
        InvokeVoid(FFI.DefineGauge, msg);
    }

    public void SetGauge(string name, double value, IDictionary<string, string>? labels = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateSetGaugeBuffer(Builder, name, value, labels);
        InvokeVoid(FFI.SetGauge, msg);
    }

    public void DefineHistogram(string name, string help, IEnumerable<double>? buckets = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateDefineHistogramBuffer(Builder, name, help, buckets);
        InvokeVoid(FFI.DefineHistogram, msg);
    }

    public void ObserveHistogram(string name, double value, IDictionary<string, string>? labels = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateObserveHistogramBuffer(Builder, name, value, labels);
        InvokeVoid(FFI.ObserveHistogram, msg);
    }

    public ICollection<FeatureDefinition> ListKnownToggles()
    {
        EnsureNotDisposed();
        return InvokeNoMsg(FFI.ListKnownToggles, Flatbuffers.GetKnownToggles);
    }

    private T Invoke<T>(NativeCall call, byte[] messageBuffer, Func<Buf, T> parse)
    {
        var buf = call(state, messageBuffer);
        try { return parse(buf); }
        finally { FFI.FreeBuf(buf); }
    }

    private void InvokeVoid(NativeCall call, byte[] messageBuffer)
    {
        var buf = call(state, messageBuffer);
        try { Flatbuffers.ParseVoidAndThrow(buf); }
        finally { FFI.FreeBuf(buf); }
    }

    private T InvokeNoMsg<T>(Func<IntPtr, Buf> call, Func<Buf, T> parse)
    {
        var buf = call(state);
        try { return parse(buf); }
        finally { FFI.FreeBuf(buf); }
    }

    private void EnsureNotDisposed()
    {
        if (disposed) throw new ObjectDisposedException(nameof(YggdrasilEngine));
    }

    internal static TRead? ReadComplex<TRead>(IntPtr ptr)
    where TRead : class
    {
        if (ptr == IntPtr.Zero) return null;

        try
        {
            string? json = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
                ? Marshal.PtrToStringAnsi(ptr)
                : Marshal.PtrToStringAuto(ptr);

            if (json is null) return null;

            var engineResponse = JsonSerializer.Deserialize<EngineResponse<TRead>>(json, jsonOptions);
            if (engineResponse is null) return null;

            if (engineResponse.StatusCode == "Error")
                throw new YggdrasilEngineException($"Error: {engineResponse.ErrorMessage}");

            return engineResponse.Value;
        }
        finally
        {
            FFI.FreeResponse(ptr);
        }
    }
}
