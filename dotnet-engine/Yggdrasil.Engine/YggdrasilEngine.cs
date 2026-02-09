using System.Text.Json;
using Google.FlatBuffers;
using yggdrasil.messaging;

namespace Yggdrasil;

public class YggdrasilEngine
{
    private CustomStrategies customStrategies;

    private JsonSerializerOptions options = new JsonSerializerOptions
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    private IntPtr state;

    public YggdrasilEngine(List<IStrategy>? strategies = null)
    {
        state = FFI.NewEngine();

        var buf = Flat.BuiltInStrategies(state);
        try
        {
            var knownStrategies = Flatbuffers.GetBuiltInStrategiesResponse(buf);
            customStrategies = new CustomStrategies(knownStrategies);

            if (strategies != null)
            {
                customStrategies.RegisterCustomStrategies(strategies);
            }
        }
        finally { Flat.FreeBuf(buf); }
    }

    public void Dispose()
    {
        FFI.FreeEngine(this.state);
        GC.SuppressFinalize(this);
    }

    public void TakeState(string json)
    {
        var buf = Flat.TakeState(state, json);
        try
        {
            var takeStateResponse = Flatbuffers.GetTakeStateResponse(buf);
            customStrategies.MapFeatures(takeStateResponse);
        }
        finally { Flat.FreeBuf(buf); }
    }

    public string GetState()
    {
        var getStatePtr = FFI.GetState(state);
        var stateObject = FFIReader.ReadComplex<object>(getStatePtr);
        return JsonSerializer.Serialize(stateObject, options);
    }

    public Response IsEnabled(string toggleName, Context context)
    {
        var customStrategyResults = customStrategies.GetCustomStrategyResults(toggleName, context);
        var messageBuffer = Flatbuffers.GetContextMessageBuffer(new FlatBufferBuilder(128), toggleName, context, customStrategyResults);
        var buf = Flat.CheckEnabled(state, messageBuffer);
        try { return Flatbuffers.GetCheckEnabledResponse(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    public Variant? GetVariant(string toggleName, Context context)
    {
        var customStrategyResults = customStrategies.GetCustomStrategyResults(toggleName, context);
        var messageBuffer = Flatbuffers.GetContextMessageBuffer(new FlatBufferBuilder(128), toggleName, context, customStrategyResults);
        var buf = Flat.CheckVariant(state, messageBuffer);
        try { return Flatbuffers.GetCheckVariantResponse(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    public MetricsBucket? GetMetrics()
    {
        var buf = Flat.GetMetrics(state);
        try { return Flatbuffers.GetMetricsBucket(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    public void DefineCounter(string name, string help)
    {
        var messageBuffer = Flatbuffers.CreateDefineCounterBuffer(new FlatBufferBuilder(128), name, help);
        var buf = Flat.DefineCounter(state, messageBuffer);
        try { Flatbuffers.ParseVoidAndThrow(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    public ICollection<FeatureDefinition> ListKnownToggles()
    {
        var buf = Flat.ListKnownToggles(state);
        try { return Flatbuffers.GetKnownToggles(buf); }
        finally { Flat.FreeBuf(buf); }
    }
}
