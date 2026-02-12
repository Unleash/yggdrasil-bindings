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

    // Cache delegates so we don't have to deal with potential method group conversion
    private static readonly NativeCall s_checkEnabled = Flat.CheckEnabled;
    private static readonly NativeCall s_checkVariant = Flat.CheckVariant;
    private static readonly NativeCall s_defineCounter = Flat.DefineCounter;
    private static readonly NativeCall s_incCounter = Flat.IncCounter;
    private static readonly NativeCall s_defineGauge = Flat.DefineGauge;
    private static readonly NativeCall s_setGauge = Flat.SetGauge;
    private static readonly NativeCall s_defineHistogram = Flat.DefineHistogram;
    private static readonly NativeCall s_observeHistogram = Flat.ObserveHistogram;

    // We can cache the parsing delegates too!
    private static readonly Func<Buf, Response> s_parseEnabled = Flatbuffers.GetCheckEnabledResponse;
    private static readonly Func<Buf, Variant?> s_parseVariant = Flatbuffers.GetCheckVariantResponse;
    private static readonly Func<Buf, MetricsBucket?> s_parseMetrics = Flatbuffers.GetMetricsBucket;
    private static readonly Func<Buf, ICollection<FeatureDefinition>> s_parseKnownToggles = Flatbuffers.GetKnownToggles;

    private static readonly JsonSerializerOptions s_jsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    private readonly CustomStrategies customStrategies;
    private IntPtr state;
    private bool disposed;

    public YggdrasilEngine(List<IStrategy>? strategies = null)
    {
        state = FFI.NewEngine();

        var buf = Flat.BuiltInStrategies();
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
            Flat.FreeBuf(buf);
        }
    }

    public void Dispose()
    {
        if (disposed) return;
        disposed = true;

        FFI.FreeEngine(state);
        state = IntPtr.Zero;

        GC.SuppressFinalize(this);
    }

    public void TakeState(string json)
    {
        EnsureNotDisposed();

        var buf = Flat.TakeState(state, json);
        try
        {
            var takeStateResponse = Flatbuffers.GetTakeStateResponse(buf);
            customStrategies.MapFeatures(takeStateResponse);
        }
        finally
        {
            Flat.FreeBuf(buf);
        }
    }

    public string GetState()
    {
        EnsureNotDisposed();

        var getStatePtr = FFI.GetState(state);
        var stateObject = FFIReader.ReadComplex<object>(getStatePtr);
        return JsonSerializer.Serialize(stateObject, s_jsonOptions);
    }

    public Response IsEnabled(string toggleName, Context context)
    {
        EnsureNotDisposed();

        var csr = customStrategies.GetCustomStrategyResults(toggleName, context);
        var msg = Flatbuffers.GetContextMessageBuffer(Builder, toggleName, context, csr);

        return Invoke(s_checkEnabled, msg, s_parseEnabled);
    }

    public Variant? GetVariant(string toggleName, Context context)
    {
        EnsureNotDisposed();

        var csr = customStrategies.GetCustomStrategyResults(toggleName, context);
        var msg = Flatbuffers.GetContextMessageBuffer(Builder, toggleName, context, csr);

        return Invoke(s_checkVariant, msg, s_parseVariant);
    }

    public MetricsBucket? GetMetrics()
    {
        EnsureNotDisposed();
        return InvokeNoMsg(Flat.GetMetrics, s_parseMetrics);
    }

    public void DefineCounter(string name, string help)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateDefineCounterBuffer(Builder, name, help);
        InvokeVoid(s_defineCounter, msg);
    }

    public void IncCounter(string name, long value = 1, IDictionary<string, string>? labels = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateIncCounterBuffer(Builder, name, value, labels);
        InvokeVoid(s_incCounter, msg);
    }

    public void DefineGauge(string name, string help)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateDefineGaugeBuffer(Builder, name, help);
        InvokeVoid(s_defineGauge, msg);
    }

    public void SetGauge(string name, double value, IDictionary<string, string>? labels = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateSetGaugeBuffer(Builder, name, value, labels);
        InvokeVoid(s_setGauge, msg);
    }

    public void DefineHistogram(string name, string help, IEnumerable<double>? buckets = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateDefineHistogramBuffer(Builder, name, help, buckets);
        InvokeVoid(s_defineHistogram, msg);
    }

    public void ObserveHistogram(string name, double value, IDictionary<string, string>? labels = null)
    {
        EnsureNotDisposed();

        var msg = Flatbuffers.CreateObserveHistogramBuffer(Builder, name, value, labels);
        InvokeVoid(s_observeHistogram, msg);
    }

    public ICollection<FeatureDefinition> ListKnownToggles()
    {
        EnsureNotDisposed();
        return InvokeNoMsg(Flat.ListKnownToggles, s_parseKnownToggles);
    }

    private T Invoke<T>(NativeCall call, byte[] messageBuffer, Func<Buf, T> parse)
    {
        var buf = call(state, messageBuffer);
        try { return parse(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    private void InvokeVoid(NativeCall call, byte[] messageBuffer)
    {
        var buf = call(state, messageBuffer);
        try { Flatbuffers.ParseVoidAndThrow(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    private T InvokeNoMsg<T>(Func<IntPtr, Buf> call, Func<Buf, T> parse)
    {
        var buf = call(state);
        try { return parse(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    private void EnsureNotDisposed()
    {
        if (disposed) throw new ObjectDisposedException(nameof(YggdrasilEngine));
    }
}
