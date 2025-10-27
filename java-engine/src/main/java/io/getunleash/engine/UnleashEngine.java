package io.getunleash.engine;

import com.google.flatbuffers.FlatBufferBuilder;
import io.getunleash.messaging.*;
import java.lang.ref.Cleaner;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.nio.ByteBuffer;
import java.time.Instant;
import java.util.*;
import java.util.stream.Stream;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class UnleashEngine {
  private static final Logger LOGGER = LoggerFactory.getLogger(UnleashEngine.class);
  private static final Cleaner CLEANER = Cleaner.create();
  private final NativeInterface nativeEngine;
  private final CustomStrategiesEvaluator customStrategiesEvaluator;

  /*
   * Default constructor for UnleashEngine. Used when no custom strategies are needed.
   */
  public UnleashEngine() {
    this(new FlatInterface(UnleashFFI.getInstance()), null, null);
  }

  /*
   * Constructor for UnleashEngine with custom strategies.
   *
   * @param customStrategies List of custom strategies to be used.
   */
  public UnleashEngine(List<IStrategy> customStrategies) {
    this(new FlatInterface(UnleashFFI.getInstance()), customStrategies, null);
  }

  /*
   * Constructor for UnleashEngine with custom strategies and a fallback strategy.
   *
   * @param customStrategies List of custom strategies to be used.
   * @param fallbackStrategy Fallback strategy to be used when no other strategy matches.
   */
  public UnleashEngine(List<IStrategy> customStrategies, IStrategy fallbackStrategy) {
    this(new FlatInterface(UnleashFFI.getInstance()), customStrategies, fallbackStrategy);
  }

  // Only visible for testing
  UnleashEngine(
      NativeInterface nativeInterface,
      List<IStrategy> customStrategies,
      IStrategy fallbackStrategy) {
    this.nativeEngine = nativeInterface;
    if (customStrategies != null && !customStrategies.isEmpty()) {
      List<String> builtInStrategies = getBuiltInStrategies();
      this.customStrategiesEvaluator =
          new CustomStrategiesEvaluator(
              customStrategies.stream(), fallbackStrategy, new HashSet<>(builtInStrategies));
    } else {
      this.customStrategiesEvaluator =
          new CustomStrategiesEvaluator(Stream.empty(), fallbackStrategy, new HashSet<>());
    }

    CLEANER.register(this, nativeEngine::freeEngine);
  }

  private static String getRuntimeHostname() {
    String hostname = System.getProperty("hostname");
    if (hostname == null) {
      try {
        hostname = InetAddress.getLocalHost().getHostName();
      } catch (UnknownHostException e) {
        hostname = "undefined";
      }
    }
    return hostname;
  }

  private static int[] buildProperties(FlatBufferBuilder builder, Map<String, String> properties) {
    List<Map.Entry<String, String>> entries = new ArrayList<>(properties.entrySet());
    List<Integer> offsets = new ArrayList<>();
    for (Map.Entry<String, String> entry : entries) {
      if (entry.getValue() == null) {
        continue;
      }
      int keyOffset = builder.createString(entry.getKey());
      int valueOffset = builder.createString(entry.getValue());
      int propOffset = PropertyEntry.createPropertyEntry(builder, keyOffset, valueOffset);
      offsets.add(propOffset);
    }
    return offsets.stream().mapToInt(Integer::intValue).toArray();
  }

  private static int[] buildCustomStrategyResults(
      FlatBufferBuilder builder, Map<String, Boolean> results) {
    List<Map.Entry<String, Boolean>> entries = new ArrayList<>(results.entrySet());
    List<Integer> offsets = new ArrayList<>();
    for (Map.Entry<String, Boolean> entry : entries) {
      if (entry.getValue() == null) {
        continue;
      }
      int keyOffset = builder.createString(entry.getKey());
      int propOffset =
          PropertyEntry.createPropertyEntry(builder, keyOffset, entry.getValue() ? 1 : 0);
      offsets.add(propOffset);
    }
    return offsets.stream().mapToInt(Integer::intValue).toArray();
  }

  private static ByteBuffer buildMessage(
      String toggleName, Context context, Map<String, Boolean> customStrategyResults) {
    ByteBuffer buffer = ByteBuffer.allocateDirect(1024);
    FlatBufferBuilder builder = new FlatBufferBuilder(buffer);

    int toggleNameOffset = builder.createString(toggleName);

    int userIdOffset = context.getUserId() != null ? builder.createString(context.getUserId()) : 0;

    int sessionIdOffset =
        context.getSessionId() != null ? builder.createString(context.getSessionId()) : 0;

    int appNameOffset =
        context.getAppName() != null ? builder.createString(context.getAppName()) : 0;

    int remoteAddressOffset =
        context.getRemoteAddress() != null ? builder.createString(context.getRemoteAddress()) : 0;

    String currentTime =
        context.getCurrentTime() != null
            ? context.getCurrentTime()
            : java.time.Instant.now().toString();
    int currentTimeOffset = builder.createString(currentTime);

    int environmentOffset =
        context.getEnvironment() != null ? builder.createString(context.getEnvironment()) : 0;

    int[] propertyOffsets = buildProperties(builder, context.getProperties());
    int[] customStrategyResultsOffsets = buildCustomStrategyResults(builder, customStrategyResults);

    String runtimeHostname = getRuntimeHostname();
    int runtimeHostnameOffset =
        runtimeHostname != null
            ? builder.createString(runtimeHostname)
            : builder.createString(getRuntimeHostname());

    int propsVec = ContextMessage.createPropertiesVector(builder, propertyOffsets);
    int customStrategyResultsVec =
        ContextMessage.createCustomStrategiesResultsVector(builder, customStrategyResultsOffsets);

    ContextMessage.startContextMessage(builder);

    if (userIdOffset != 0) ContextMessage.addUserId(builder, userIdOffset);
    if (sessionIdOffset != 0) ContextMessage.addSessionId(builder, sessionIdOffset);
    if (appNameOffset != 0) ContextMessage.addAppName(builder, appNameOffset);
    if (environmentOffset != 0) ContextMessage.addEnvironment(builder, environmentOffset);
    if (remoteAddressOffset != 0) ContextMessage.addRemoteAddress(builder, remoteAddressOffset);
    if (runtimeHostnameOffset != 0)
      ContextMessage.addRuntimeHostname(builder, runtimeHostnameOffset);

    ContextMessage.addCurrentTime(builder, currentTimeOffset);
    ContextMessage.addToggleName(builder, toggleNameOffset);

    if (propertyOffsets.length > 0) {
      ContextMessage.addProperties(builder, propsVec);
    }

    if (customStrategyResultsOffsets.length > 0) {
      ContextMessage.addCustomStrategiesResults(builder, customStrategyResultsVec);
    }

    int ctx = ContextMessage.endContextMessage(builder);
    builder.finish(ctx);
    return builder.dataBuffer();
  }

  public void takeState(String clientFeatures) throws YggdrasilInvalidInputException {
    if (clientFeatures == null) {
      return;
    }
    try {
      TakeStateResponse takeStateResponse = this.nativeEngine.takeState(clientFeatures);
      customStrategiesEvaluator.loadStrategiesFor(takeStateResponse);
    } catch (RuntimeException e) {
      throw new YggdrasilInvalidInputException("Failed to take state:", e);
    }
  }

  /**
   * Used to evaluate a feature toggle.
   *
   * @param toggleName The name of the feature toggle to evaluate.
   * @param context The context in which to evaluate the feature toggle.
   * @return A response containing the evaluation result and impression data.
   * @throws YggdrasilInvalidInputException If there is an error evaluating the feature toggle.
   */
  public FlatResponse<Boolean> isEnabled(String toggleName, Context context)
      throws YggdrasilInvalidInputException {
    try {
      Map<String, Boolean> strategyResults = customStrategiesEvaluator.eval(toggleName, context);
      ByteBuffer contextBytes = buildMessage(toggleName, context, strategyResults);
      Response response = this.nativeEngine.checkEnabled(contextBytes);

      if (response.error() != null) {
        String error = response.error();
        throw new YggdrasilInvalidInputException(error);
      }

      if (response.hasEnabled()) {
        return new FlatResponse<>(response.impressionData(), response.enabled());
      } else {
        return new FlatResponse<>(response.impressionData(), null);
      }
    } catch (RuntimeException e) {
      LOGGER.warn("Could not check if toggle is enabled: {}", e.getMessage(), e);
      return new FlatResponse<>(false, null);
    }
  }

  /**
   * Used to get the active variant for a toggle and the current context.
   *
   * @param toggleName The name of the toggle to check.
   * @param context The context to use for the toggle check.
   * @return A FlatResponse containing the impression data and the active variant.
   * @throws YggdrasilInvalidInputException If the input is invalid.
   */
  public FlatResponse<VariantDef> getVariant(String toggleName, Context context)
      throws YggdrasilInvalidInputException {
    try {
      Map<String, Boolean> strategyResults = customStrategiesEvaluator.eval(toggleName, context);
      ByteBuffer contextBytes = buildMessage(toggleName, context, strategyResults);

      Variant variant = this.nativeEngine.checkVariant(contextBytes);
      if (variant.name() != null) {
        Payload payload = null;

        VariantPayload variantPayload = variant.payload();

        if (variantPayload != null) {
          payload = new Payload();
          payload.setType(variant.payload().payloadType());
          payload.setValue(variant.payload().value());
        }

        if (variant.error() != null) {
          String error = variant.error();
          throw new YggdrasilInvalidInputException(error);
        }

        return new FlatResponse<>(
            variant.impressionData(),
            new VariantDef(variant.name(), payload, variant.enabled(), variant.featureEnabled()));
      } else {
        return new FlatResponse<>(false, null);
      }
    } catch (RuntimeException e) {
      LOGGER.warn("Could not get variant for toggle '{}': {}", toggleName, e.getMessage(), e);
      return new FlatResponse<>(false, null);
    }
  }

  /**
   * Returns a list of built-in strategies that Yggdrasil supports.
   *
   * @return A list of built-in strategies.
   */
  public List<String> getBuiltInStrategies() {
    BuiltInStrategies builtInStrategies = FlatInterface.getBuiltInStrategies();
    if (builtInStrategies != null) {
      List<String> builtInStrategiesNames = new ArrayList<>(builtInStrategies.valuesLength());
      for (int i = 0; i < builtInStrategies.valuesLength(); i++) {
        builtInStrategiesNames.add(builtInStrategies.values(i));
      }
      return builtInStrategiesNames;
    } else {
      return Collections.emptyList();
    }
  }

  /**
   * Which version of Yggdrasil is being used.
   *
   * @return The version string.
   */
  public static String getCoreVersion() {
    return UnleashFFI.getInstance().getCoreVersion().getString(0);
  }

  public String getState() {
    return this.nativeEngine.getState();
  }

  /**
   * Get all toggles that is loaded in the engine.
   *
   * @return A list of FeatureDef objects representing the loaded toggles.
   */
  public List<io.getunleash.engine.FeatureDef> listKnownToggles() {
    try {
      var knownToggles = this.nativeEngine.listKnownToggles();
      var toggleList = new ArrayList<FeatureDef>(knownToggles.itemsLength());
      for (int i = 0; i < knownToggles.itemsLength(); i++) {
        var tempFeature = knownToggles.items(i);
        toggleList.add(
            new FeatureDef(
                tempFeature.name(),
                tempFeature.type(),
                tempFeature.project(),
                tempFeature.enabled()));
      }
      return toggleList;
    } catch (RuntimeException e) {
      LOGGER.warn("Could not list known toggles: {}", e.getMessage(), e);
      return Collections.emptyList();
    }
  }

  /**
   * Get the metrics for the engine. Metrics are collected during evaluation of feature toggles as
   * well as when getVariant is called.
   *
   * @return A MetricsBucket object representing the engine's metrics.
   */
  public MetricsBucket getMetrics() {
    var metrics = this.nativeEngine.getMetrics();
    Map<String, FeatureCount> toggles = new HashMap<>();
    for (int i = 0; i < metrics.togglesLength(); i++) {
      ToggleEntry toggleEntry = metrics.toggles(i);
      ToggleStats stats = toggleEntry.value();

      Map<String, Long> variants = new HashMap<>();
      for (int j = 0; j < stats.variantsLength(); j++) {
        VariantEntry variant = stats.variants(j);
        variants.put(variant.key(), variant.value());
      }
      FeatureCount featureCount = new FeatureCount(stats.yes(), stats.no(), variants);

      toggles.put(toggleEntry.key(), featureCount);
    }
    return new MetricsBucket(
        Instant.ofEpochMilli(metrics.start()), Instant.ofEpochMilli(metrics.stop()), toggles);
  }

  // Only visible for testing.
  Map<String, Boolean> customStrategiesEvaluatorEval(String featureName, Context context) {
    return this.customStrategiesEvaluator.eval(featureName, context);
  }

  List<CustomStrategiesEvaluator.MappedStrategy> customStrategiesForFeature(String featureName) {
    return this.customStrategiesEvaluator
        .getFeatureStrategies()
        .getOrDefault(featureName, Collections.emptyList());
  }
}
