namespace Yggdrasil;

using System;
using System.Text.Json;

internal class CustomStrategies
{
    private JsonSerializerOptions options = new JsonSerializerOptions
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    private string[]? knownStrategies = null;

    private Dictionary<string, IStrategy> strategies = new Dictionary<string, IStrategy>();
    private Dictionary<string, MappedFeature>? mappedFeatures = null;

    internal CustomStrategies(string[]? knownStrategies)
    {
        this.knownStrategies = knownStrategies;
    }

    private static readonly IReadOnlyDictionary<string, bool> noCustomStrategies = new Dictionary<string, bool>(0);

    private bool IsCustomStrategy(StrategyDefinition strategy)
    {
        return !knownStrategies?.Contains(strategy.Name) ?? false;
    }

    private List<MappedStrategy> MapCustomStrategies(List<StrategyDefinition>? strategies)
    {
        if (strategies == null)
        {
            return new List<MappedStrategy>();
        }

        return strategies
            .Where(IsCustomStrategy)
            .Where(definition => this.strategies?.ContainsKey(definition.Name) ?? false)
            .Select((definition, index) =>
                new MappedStrategy(
                    index,
                    definition.Name,
                    definition.Parameters ?? new Dictionary<string, string>(),
                    this.strategies[definition.Name]))
            .ToList();
    }

    internal void MapFeatures(Dictionary<string, List<StrategyDefinition>> featureStrategies)
    {
        mappedFeatures = new Dictionary<string, MappedFeature>();
        foreach (var kvp in featureStrategies)
        {
            mappedFeatures.Add(kvp.Key, MapFeature(kvp.Key, kvp.Value));
        }
    }
    internal void MapFeatures(string json)
    {
        var features = JsonSerializer.Deserialize<FeatureCollection>(json, options)?.Features;
        mappedFeatures = features?
            .Select(feature => new MappedFeature(feature, MapCustomStrategies(feature.Strategies)))
            .ToDictionary(feature => feature.Name);
    }

    private MappedFeature MapFeature(string name, List<StrategyDefinition>? strategies)
    {
        return new MappedFeature(new Feature() { Name = name }, MapCustomStrategies(strategies ?? new List<StrategyDefinition>()));
    }

    internal void RegisterCustomStrategies(List<IStrategy> strategies)
    {
        this.strategies = strategies.ToDictionary(strategy => strategy.Name);
    }

    internal string GetCustomStrategyPayload(string toggleName, Context context)
    {
        MappedFeature? feature = null;
        mappedFeatures?.TryGetValue(toggleName, out feature);
        if (feature == null)
        {
            return "{}";
        }

        var strategies = feature.Strategies
            .ToDictionary(strategy => strategy.ResultName,
                strategy => strategy.IsEnabled(context));

        return JsonSerializer.Serialize(strategies, options);
    }

    internal IReadOnlyDictionary<string, bool> GetCustomStrategyResults(string toggleName, Context context)
    {
        MappedFeature? feature = null;
        mappedFeatures?.TryGetValue(toggleName, out feature);
        if (feature == null || feature.Strategies.Count == 0)
        {
            return noCustomStrategies;
        }

        var customStrategyResults = new Dictionary<string, bool>(feature.Strategies.Count);
        for (int i = 0; i < feature.Strategies.Count; i++)
        {
            var strategy = feature.Strategies[i];
            customStrategyResults[strategy.ResultName] = strategy.IsEnabled(context);
        }
        return customStrategyResults;
    }
}
