using System.Text.Json;
using Google.FlatBuffers;

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

    public bool ShouldEmitImpressionEvent(string featureName)
    {
        var shouldEmitImpressionEventPtr = FFI.ShouldEmitImpressionEvent(state, featureName);
        var shouldEmitImpressionEvent = FFIReader.ReadPrimitive<bool>(shouldEmitImpressionEventPtr);

        return shouldEmitImpressionEvent ?? false;
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

    public bool? IsEnabled(string toggleName, Context context)
    {
        var customStrategyResults = customStrategies.GetCustomStrategyResults(toggleName, context);
        var builder = new FlatBufferBuilder(128);
        var array = Flatbuffers.GetContextMessageBuffer(builder, toggleName, context, customStrategyResults);
        var buf = Flat.CheckEnabled(state, array);
        try { return Flatbuffers.GetCheckEnabledResponse(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    public Variant? GetVariant(string toggleName, Context context)
    {
        var customStrategyResults = customStrategies.GetCustomStrategyResults(toggleName, context);
        var builder = new FlatBufferBuilder(128);
        var array = Flatbuffers.GetContextMessageBuffer(builder, toggleName, context, customStrategyResults);
        var buf = Flat.CheckVariant(state, array);
        try { return Flatbuffers.GetCheckVariantResponse(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    public MetricsBucket? GetMetrics()
    {
        var buf = Flat.GetMetrics(state);
        try { return Flatbuffers.GetMetricsBucket(buf); }
        finally { Flat.FreeBuf(buf); }
    }

    public ICollection<FeatureDefinition> ListKnownToggles()
    {
        var buf = Flat.ListKnownToggles(state);
        try { return Flatbuffers.GetKnownToggles(buf); }
        finally { Flat.FreeBuf(buf); }
    }
}
