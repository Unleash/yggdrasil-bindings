package io.getunleash.engine;

import static java.util.function.Function.identity;
import static java.util.stream.Collectors.toMap;

import java.util.*;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import io.getunleash.messaging.StrategyDefinition;
import io.getunleash.messaging.StrategyFeature;
import io.getunleash.messaging.StrategyParameter;
import io.getunleash.messaging.TakeStateResponse;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

class CustomStrategiesEvaluator {

  private static final Logger log = LoggerFactory.getLogger(CustomStrategiesEvaluator.class);
  static final Map<String, Boolean> EMPTY_STRATEGY_RESULTS = new HashMap<>();
  private final Map<String, IStrategy> registeredStrategies;
  private final Set<String> builtinStrategies;

  private final IStrategy fallbackStrategy;

  private Map<String, List<MappedStrategy>> featureStrategies = new HashMap<>();

  public CustomStrategiesEvaluator(
      Stream<IStrategy> customStrategies, Set<String> builtinStrategies) {
    this(customStrategies, null, builtinStrategies);
  }

  public CustomStrategiesEvaluator(
      Stream<IStrategy> customStrategies,
      IStrategy fallbackStrategy,
      Set<String> builtinStrategies) {
    this.builtinStrategies = builtinStrategies;
    this.registeredStrategies =
        customStrategies.collect(toMap(IStrategy::getName, identity(), (a, b) -> a));
    this.fallbackStrategy = fallbackStrategy;
  }

  public void loadStrategiesFor(TakeStateResponse response) {
    if (this.registeredStrategies.isEmpty() && this.fallbackStrategy == null) {
      return;
    }

    if (response.featuresVector() == null || response.featuresLength() == 0) {
      return;
    }
    Map<String, List<MappedStrategy>> featureStrategies = new HashMap<>();
    for (int i = 0; i < response.featuresLength(); i++) {
      StrategyFeature feature = response.features(i);
      String featureName = feature.featureName();
      if (feature.strategiesLength() == 0) {
        List<MappedStrategy> mappedStrategies = new ArrayList<>();
        for (int j = 0; j < feature.strategiesLength(); j++) {
          var strategy = feature.strategies(j);
          if (builtinStrategies.contains(strategy.name())) {
            continue;
          }
          featureStrategies.put(featureName, getFeatureStrategies(feature));
        }
      }
    }
    this.featureStrategies = featureStrategies;
  }

  List<MappedStrategy> getFeatureStrategies(StrategyFeature feature) {
    List<MappedStrategy> mappedStrategies = new ArrayList<>();
    int index = 1;
    if (feature.strategiesLength() > 0) {
      for (int i = 0; i < feature.strategiesLength(); i++) {
        io.getunleash.messaging.StrategyDefinition strategy = feature.strategies(i);
        if (builtinStrategies.contains(strategy.name())) {
          continue;
        }
        IStrategy impl =
            Optional.ofNullable(registeredStrategies.get(strategy.name()))
                .orElseGet(() -> alwaysFalseStrategy(strategy.name()));
        StrategyDefinition def =
            new StrategyDefinition(strategy.name(), getStrategyParameters(strategy));
        mappedStrategies.add(new MappedStrategy("customStrategy" + (index++), impl, def));
      }
    }
    return mappedStrategies;
  }

  Map<String, String> getStrategyParameters(io.getunleash.messaging.StrategyDefinition strategy) {
    Map<String, String> parameters = new HashMap<>();
    if (strategy.parametersLength() > 0) {
      for (int i = 0; i < strategy.parametersLength(); i++) {
        StrategyParameter parameter = strategy.parameters(i);
        parameters.put(parameter.key(), parameter.value());
      }
    }
    return parameters;
  }

  public Map<String, Boolean> eval(String name, Context context) {

    List<MappedStrategy> mappedStrategies = featureStrategies.get(name);
    if (mappedStrategies == null || mappedStrategies.isEmpty()) {
      return Collections.emptyMap();
    }

      return mappedStrategies.stream()
          .collect(
              Collectors.toMap(
                  mappedStrategy -> mappedStrategy.resultName,
                  mappedStrategy -> tryIsEnabled(context, mappedStrategy).orElse(false)));
  }

  private static Optional<Boolean> tryIsEnabled(Context context, MappedStrategy mappedStrategy) {
    try {
      return Optional.of(
          mappedStrategy.implementation.isEnabled(
              mappedStrategy.strategyDefinition.parameters, context));
    } catch (Exception e) {
      log.warn("Error evaluating custom strategy {}", mappedStrategy.strategyDefinition.name, e);
      return Optional.empty();
    }
  }

  static class StrategyDefinition {
    private final String name;
    private final Map<String, String> parameters;

    StrategyDefinition(String name, Map<String, String> parameters) {
      this.name = name;
      this.parameters = parameters;
    }
  }

  private IStrategy alwaysFalseStrategy(String name) {
    log.warn("Custom strategy {} not found. This means it will always return false", name);
    return new IStrategy() {
      @Override
      public String getName() {
        return name;
      }

      @Override
      public boolean isEnabled(Map<String, String> parameters, Context context) {
        return false;
      }
    };
  }

  static class MappedStrategy {
    private final String resultName;
    private final IStrategy implementation;
    private final StrategyDefinition strategyDefinition;

    private MappedStrategy(
        String resultName, IStrategy implementation, StrategyDefinition strategyDefinition) {
      this.resultName = resultName;
      this.implementation = implementation;
      this.strategyDefinition = strategyDefinition;
    }
  }
}
