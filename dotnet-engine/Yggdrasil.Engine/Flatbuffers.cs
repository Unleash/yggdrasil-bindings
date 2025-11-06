using System.Collections.ObjectModel;
using System.Runtime.InteropServices;
using Google.FlatBuffers;
using yggdrasil.messaging;
using Yggdrasil;
using StrategyDefinition = Yggdrasil.StrategyDefinition;
using Variant = Yggdrasil.Variant;
using FlatVariant = yggdrasil.messaging.Variant;

public static class Flatbuffers
{
    private static byte[] ReadBuffer(Buf buf)
    {
        if (buf.len == 0 || buf.ptr == IntPtr.Zero)
            return Array.Empty<byte>();

        byte[] managed = new byte[buf.len];
        Marshal.Copy(buf.ptr, managed, 0, (int)buf.len);
        return managed;
    }

    public static byte[] GetContextMessageBuffer(FlatBufferBuilder builder, string featureName, Context context, Dictionary<string, bool> customStrategyResults)
    {
        var toggleName = builder.CreateString(featureName);
        var appName = builder.CreateString(context.AppName);
        var currentTimeOffset = builder.CreateString(context.CurrentTime.HasValue ? context.CurrentTime.Value.ToString("O") : null);
        var environment = builder.CreateString(context.Environment);
        var remoteAddress = builder.CreateString(context.RemoteAddress);
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
        ContextMessage.AddSessionId(builder, sessionId);
        ContextMessage.AddUserId(builder, userId);
        ContextMessage.AddProperties(builder, propertiesVector);

        var contextMessage = ContextMessage.EndContextMessage(builder);
        builder.Finish(contextMessage.Value);
        return builder.SizedByteArray();
    }

    internal static VectorOffset CreatePropertiesVector(FlatBufferBuilder builder, Context context)
    {
        var propertyEntries = new Offset<PropertyEntry>[context.Properties?.Count ?? 0];

        for (var i = 0; i < context.Properties?.Count; i++)
        {
            var kvp = context.Properties.ElementAt(i);
            propertyEntries[i] = PropertyEntry.CreatePropertyEntry(builder, builder.CreateString(kvp.Key), builder.CreateString(kvp.Value));
        }
        return ContextMessage.CreatePropertiesVector(builder, propertyEntries);
    }

    internal static VectorOffset CreateCustomStrategiesVector(FlatBufferBuilder builder, Dictionary<string, bool> customStrategyResults)
    {
        var strategyEntries = new Offset<CustomStrategyResult>[customStrategyResults.Count];
        for (var i = 0; i < customStrategyResults.Count; i++)
        {
            var kvp = customStrategyResults.ElementAt(i);

            strategyEntries[i] = CustomStrategyResult.CreateCustomStrategyResult(builder, builder.CreateString(kvp.Key), kvp.Value);
        }
        return ContextMessage.CreateCustomStrategiesResultsVector(builder, strategyEntries);
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