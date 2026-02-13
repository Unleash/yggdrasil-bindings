using System.Collections.ObjectModel;
using System.Runtime.InteropServices;
using Google.FlatBuffers;
using yggdrasil.messaging;
using Yggdrasil;
using StrategyDefinition = Yggdrasil.StrategyDefinition;
using Variant = Yggdrasil.Variant;
using FlatVariant = yggdrasil.messaging.Variant;
using System.Net;
using System.Linq.Expressions;

public static class Flatbuffers
{
    private static readonly string hostname = Environment.GetEnvironmentVariable("hostname") ?? Dns.GetHostName();

    private static byte[] ReadBuffer(Buf buf)
    {
        if (buf.len == 0 || buf.ptr == IntPtr.Zero)
            return Array.Empty<byte>();

        byte[] managed = new byte[buf.len];
        Marshal.Copy(buf.ptr, managed, 0, (int)buf.len);
        return managed;
    }

    public static byte[] GetContextMessageBuffer(FlatBufferBuilder builder, string featureName, Context context, IReadOnlyDictionary<string, bool> customStrategyResults)
    {
        var toggleName = builder.CreateString(featureName);
        var appName = builder.CreateString(context.AppName);
        var currentTimeOffset = builder.CreateString(context.CurrentTime.HasValue ? context.CurrentTime.Value.ToString("O") : null);
        var environment = builder.CreateString(context.Environment);
        var remoteAddress = builder.CreateString(context.RemoteAddress);
        var hostname = builder.CreateString(Flatbuffers.hostname);
        var sessionId = builder.CreateString(context.SessionId);
        var userId = builder.CreateString(context.UserId);
        var propertiesVector = CreatePropertiesVector(builder, context);
        var customStrategiesVector = CreateCustomStrategiesVector(builder, customStrategyResults);

        ContextMessage.StartContextMessage(builder);
        ContextMessage.AddToggleName(builder, toggleName);
        ContextMessage.AddAppName(builder, appName);
        ContextMessage.AddCurrentTime(builder, currentTimeOffset);
        ContextMessage.AddEnvironment(builder, environment);
        ContextMessage.AddCustomStrategiesResults(builder, customStrategiesVector);
        ContextMessage.AddRemoteAddress(builder, remoteAddress);
        ContextMessage.AddRuntimeHostname(builder, hostname);
        ContextMessage.AddSessionId(builder, sessionId);
        ContextMessage.AddUserId(builder, userId);
        ContextMessage.AddProperties(builder, propertiesVector);

        var contextMessage = ContextMessage.EndContextMessage(builder);
        builder.Finish(contextMessage.Value);
        return builder.SizedByteArray();
    }

    public static byte[] CreateDefineCounterBuffer(FlatBufferBuilder builder, string name, string help)
    {
        var nameOffset = builder.CreateString(name);
        var helpOffset = builder.CreateString(help);

        DefineCounter.StartDefineCounter(builder);
        DefineCounter.AddName(builder, nameOffset);
        DefineCounter.AddHelp(builder, helpOffset);

        var defineCounterMessage = DefineCounter.EndDefineCounter(builder);
        builder.Finish(defineCounterMessage.Value);
        return builder.SizedByteArray();
    }

    public static byte[] CreateIncCounterBuffer(FlatBufferBuilder builder, string name, long value, IDictionary<string, string>? labels = null)
    {
        var nameOffset = builder.CreateString(name);
        var labelsOffset = CreateSampleLabelsVector(builder, labels);

        IncCounter.StartIncCounter(builder);
        IncCounter.AddName(builder, nameOffset);
        IncCounter.AddValue(builder, value);
        if (labelsOffset.HasValue)
        {
            IncCounter.AddLabels(builder, labelsOffset.Value);
        }

        var incCounterMessage = IncCounter.EndIncCounter(builder);
        builder.Finish(incCounterMessage.Value);
        return builder.SizedByteArray();
    }

    public static byte[] CreateDefineGaugeBuffer(FlatBufferBuilder builder, string name, string help)
    {
        var nameOffset = builder.CreateString(name);
        var helpOffset = builder.CreateString(help);

        DefineGauge.StartDefineGauge(builder);
        DefineGauge.AddName(builder, nameOffset);
        DefineGauge.AddHelp(builder, helpOffset);

        var defineGaugeMessage = DefineGauge.EndDefineGauge(builder);
        builder.Finish(defineGaugeMessage.Value);
        return builder.SizedByteArray();
    }

    public static byte[] CreateSetGaugeBuffer(FlatBufferBuilder builder, string name, double value, IDictionary<string, string>? labels = null)
    {
        var nameOffset = builder.CreateString(name);
        var labelsOffset = CreateSampleLabelsVector(builder, labels);

        SetGauge.StartSetGauge(builder);
        SetGauge.AddName(builder, nameOffset);
        SetGauge.AddValue(builder, value);
        if (labelsOffset.HasValue)
        {
            SetGauge.AddLabels(builder, labelsOffset.Value);
        }

        var setGaugeMessage = SetGauge.EndSetGauge(builder);
        builder.Finish(setGaugeMessage.Value);
        return builder.SizedByteArray();
    }

    public static byte[] CreateDefineHistogramBuffer(FlatBufferBuilder builder, string name, string help, IEnumerable<double>? buckets = null)
    {
        var nameOffset = builder.CreateString(name);
        var helpOffset = builder.CreateString(help);
        var bucketArray = (buckets ?? Enumerable.Empty<double>()).ToArray();
        var bucketsOffset = bucketArray.Length > 0
            ? DefineHistogram.CreateBucketsVector(builder, bucketArray)
            : default(VectorOffset);

        DefineHistogram.StartDefineHistogram(builder);
        DefineHistogram.AddName(builder, nameOffset);
        DefineHistogram.AddHelp(builder, helpOffset);
        if (bucketArray.Length > 0)
        {
            DefineHistogram.AddBuckets(builder, bucketsOffset);
        }

        var defineHistogramMessage = DefineHistogram.EndDefineHistogram(builder);
        builder.Finish(defineHistogramMessage.Value);
        return builder.SizedByteArray();
    }

    public static byte[] CreateObserveHistogramBuffer(FlatBufferBuilder builder, string name, double value, IDictionary<string, string>? labels = null)
    {
        var nameOffset = builder.CreateString(name);
        var labelsOffset = CreateSampleLabelsVector(builder, labels);

        ObserveHistogram.StartObserveHistogram(builder);
        ObserveHistogram.AddName(builder, nameOffset);
        ObserveHistogram.AddValue(builder, value);
        if (labelsOffset.HasValue)
        {
            ObserveHistogram.AddLabels(builder, labelsOffset.Value);
        }

        var observeHistogramMessage = ObserveHistogram.EndObserveHistogram(builder);
        builder.Finish(observeHistogramMessage.Value);
        return builder.SizedByteArray();
    }

    private static VectorOffset? CreateSampleLabelsVector(FlatBufferBuilder builder, IDictionary<string, string>? labels)
    {
        if (labels == null || labels.Count == 0)
        {
            return null;
        }

        var labelEntries = new Offset<SampleLabelEntry>[labels.Count];
        var index = 0;
        foreach (var kvp in labels)
        {
            labelEntries[index] = SampleLabelEntry.CreateSampleLabelEntry(
                builder,
                builder.CreateString(kvp.Key),
                builder.CreateString(kvp.Value)
            );
            index++;
        }

        return IncCounter.CreateLabelsVector(builder, labelEntries);
    }

    internal static VectorOffset CreatePropertiesVector(FlatBufferBuilder builder, Context context)
    {
        var props = context.Properties;
        if (props is null || props.Count == 0)
            return default; // no vector

        var entries = new Offset<PropertyEntry>[props.Count];
        int i = 0;

        foreach (var kvp in props)
        {
            entries[i++] = PropertyEntry.CreatePropertyEntry(
                builder,
                builder.CreateString(kvp.Key),
                builder.CreateString(kvp.Value));
        }

        return ContextMessage.CreatePropertiesVector(builder, entries);
    }

    internal static VectorOffset CreateCustomStrategiesVector(
        FlatBufferBuilder builder,
        IReadOnlyDictionary<string, bool> customStrategyResults)
    {
        if (customStrategyResults.Count == 0)
            return default;

        var entries = new Offset<CustomStrategyResult>[customStrategyResults.Count];
        int i = 0;

        foreach (var kvp in customStrategyResults)
        {
            entries[i++] = CustomStrategyResult.CreateCustomStrategyResult(
                builder,
                builder.CreateString(kvp.Key),
                kvp.Value);
        }

        return ContextMessage.CreateCustomStrategiesResultsVector(builder, entries);
    }

    internal static Variant? GetCheckVariantResponse(Buf buf)
    {
        var buffer = ReadBuffer(buf);
        var response = FlatVariant.GetRootAsVariant(new ByteBuffer(buffer));
        if (response.Error != null)
        {
            throw new YggdrasilEngineException(response.Error);
        }
        if (response.Name != null)
        {
            return new Variant(
                response.Name,
                response.Payload != null ? new Payload(response.Payload.Value.PayloadType, response.Payload.Value.Value) : null,
                response.Enabled,
                response.FeatureEnabled
            );
        }
        return null;
    }

    internal static Response GetCheckEnabledResponse(Buf buf)
    {
        var buffer = ReadBuffer(buf);
        var response = Response.GetRootAsResponse(new ByteBuffer(buffer));
        if (response.Error != null)
        {
            throw new YggdrasilEngineException(response.Error);
        }

        return response;
    }

    internal static string[] GetBuiltInStrategiesResponse(Buf buf)
    {
        var response = ReadBuffer(buf);
        var builtInStrategies = BuiltInStrategies.GetRootAsBuiltInStrategies(new ByteBuffer(response));
        return Enumerable.Range(0, builtInStrategies.ValuesLength)
            .Select(i => builtInStrategies.Values(i)).ToArray();
    }

    internal static Dictionary<string, List<StrategyDefinition>> GetTakeStateResponse(Buf buf)
    {
        var response = ReadBuffer(buf);
        var takeStateResponse = TakeStateResponse.GetRootAsTakeStateResponse(new ByteBuffer(response));
        if (takeStateResponse.Error != null)
        {
            throw new YggdrasilEngineException(takeStateResponse.Error);
        }

        return Enumerable.Range(0, takeStateResponse.FeaturesLength)
            .Select(i =>
            {
                var feature = takeStateResponse.Features(i)!.Value;
                return new { Name = feature.FeatureName, Strategies = MapFeatureStrategies(feature) };
            })
            .ToDictionary(kvp => kvp.Name, kvp => kvp.Strategies);
    }

    private static List<StrategyDefinition> MapFeatureStrategies(StrategyFeature feature)
    {
        return Enumerable.Range(0, feature.StrategiesLength)
                    .Select(s =>
                    {
                        var strategy = feature.Strategies(s)!.Value;
                        return new StrategyDefinition()
                        {
                            Name = strategy.Name,
                            Parameters = Enumerable
                                .Range(0, strategy.ParametersLength)
                                .Select(p =>
                                    {
                                        var param = strategy.Parameters(p)!.Value;
                                        return new { param.Key, param.Value };
                                    })
                                .ToDictionary(kvp => kvp.Key, kvp => kvp.Value)
                        };
                    }).ToList();
    }


    public static ICollection<FeatureDefinition> GetKnownToggles(Buf buf)
    {
        var response = ReadBuffer(buf);
        var knownTogglesResponse = FeatureDefs.GetRootAsFeatureDefs(new ByteBuffer(response));
        return Enumerable.Range(0, knownTogglesResponse.ItemsLength)
            .Select(i =>
            {
                var item = knownTogglesResponse.Items(i)!.Value;
                return new FeatureDefinition(item.Name, item.Project, item.Type);
            }).ToList();

    }
    public static MetricsBucket GetMetricsBucket(Buf buf)
    {
        var response = ReadBuffer(buf);
        var metricsResponse = MetricsResponse.GetRootAsMetricsResponse(new ByteBuffer(response));
        return new MetricsBucket(
            GetMetricsFeatureCounts(metricsResponse),
            DateTimeOffset.FromUnixTimeMilliseconds(metricsResponse.Start),
            DateTimeOffset.FromUnixTimeMilliseconds(metricsResponse.Stop)
        );
    }

    public static void ParseVoidAndThrow(Buf buf)
    {
        var response = ReadBuffer(buf);
        var voidResponse = VoidResponse.GetRootAsVoidResponse(new ByteBuffer(response));
        if (voidResponse.Error != null)
        {
            throw new YggdrasilEngineException(voidResponse.Error);
        }
    }

    private static Dictionary<string, FeatureCount> GetMetricsFeatureCounts(MetricsResponse response)
    {
        return Enumerable.Range(0, response.TogglesLength)
            .Select(x =>
            {
                var featureKvp = response.Toggles(x)!.Value;
                return new { Name = featureKvp.Key, Feature = GetFeatureCount(featureKvp) };
            })
            .ToDictionary(
                x => x.Name,
                v => v.Feature
            );
    }

    private static FeatureCount GetFeatureCount(ToggleEntry response)
    {
        var entry = response.Value!.Value;
        return new FeatureCount(
            entry.Yes,
            entry.No,
            GetVariantCounts(entry)
        );
    }

    private static Dictionary<string, long> GetVariantCounts(ToggleStats stats)
    {
        return Enumerable
            .Range(0, stats.VariantsLength)
            .Select(i =>
            {
                var variant = stats.Variants(i)!.Value;
                long variantValue = variant.Value;
                return new { variant.Key, Value = variantValue };
            })
            .ToDictionary(x => x.Key, v => v.Value);
    }

}
