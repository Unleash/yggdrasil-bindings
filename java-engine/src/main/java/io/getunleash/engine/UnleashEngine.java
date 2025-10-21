package io.getunleash.engine;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import com.google.flatbuffers.FlatBufferBuilder;
import java.lang.ref.Cleaner;
import java.net.InetAddress;
import java.net.UnknownHostException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.time.ZoneOffset;
import java.time.ZonedDateTime;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;

import com.sun.jna.Memory;
import com.sun.jna.Pointer;
import messaging.BuiltInStrategies;
import messaging.ContextMessage;
import messaging.FeatureDefs;
import messaging.MetricsResponse;
import messaging.PropertyEntry;
import messaging.Response;
import messaging.ToggleEntry;
import messaging.ToggleStats;
import messaging.Variant;
import messaging.VariantEntry;
import messaging.VariantPayload;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class UnleashEngine {
  private static final String EMPTY_STRATEGY_RESULTS = "{}";
  private static final Logger log = LoggerFactory.getLogger(UnleashEngine.class);
  private static final Cleaner cleaner = Cleaner.create();
  private final UnleashFFI yggdrasil;
  private final Pointer enginePointer;
  private final ObjectMapper mapper;
  private final CustomStrategiesEvaluator customStrategiesEvaluator;

  public UnleashEngine() {
    this(UnleashFFI.getInstance(), null, null);
  }

  public UnleashEngine(List<IStrategy> customStrategies) {
    this(UnleashFFI.getInstance(), customStrategies, null);
  }

  public UnleashEngine(List<IStrategy> customStrategies, IStrategy fallbackStrategy) {
    this(UnleashFFI.getInstance(), customStrategies, fallbackStrategy);
  }

  // Only visible for testing
  UnleashEngine(UnleashFFI ffi, List<IStrategy> customStrategies, IStrategy fallbackStrategy) {
    yggdrasil = ffi;
    this.enginePointer = yggdrasil.newEngine();
    this.mapper = new ObjectMapper();
    this.mapper.registerModule(new JavaTimeModule());
    this.mapper.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
    if (customStrategies != null && !customStrategies.isEmpty()) {
      List<String> builtInStrategies = getBuiltInStrategies();
      this.customStrategiesEvaluator =
          new CustomStrategiesEvaluator(
              customStrategies.stream(), fallbackStrategy, new HashSet<String>(builtInStrategies));
    } else {
      this.customStrategiesEvaluator =
          new CustomStrategiesEvaluator(Stream.empty(), fallbackStrategy, new HashSet<String>());
    }

    Instant now = Instant.now();

    cleaner.register(this, () -> yggdrasil.freeEngine(enginePointer));
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

  private static byte[] buildMessage(
      String toggleName, Context context, Map<String, Boolean> customStrategyResults) {
    FlatBufferBuilder builder = new FlatBufferBuilder(1024);

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
    return builder.sizedByteArray();
  }

  public void takeState(String clientFeatures) throws YggdrasilInvalidInputException {
    try {
      yggdrasil.takeState(this.enginePointer, toUtf8Pointer(clientFeatures));
      customStrategiesEvaluator.loadStrategiesFor(clientFeatures);
    } catch (RuntimeException e) {
      throw new YggdrasilInvalidInputException("Failed to take state:", e);
    }
  }

  public String getState() {
    return yggdrasil.getState(this.enginePointer);
  }

  public List<FeatureDef> listKnownToggles() {
    try {
      Pointer featureDefsPointer = yggdrasil.listKnownToggles(this.enginePointer);
        ByteBuffer featureDefsBuffer = ByteBuffer
      List<FeatureDef> defs = new ArrayList<>(featureDefs.itemsLength());
      for (int i = 0; i < featureDefs.itemsLength(); i++) {
        FeatureDef featureDef =
            new FeatureDef(
                featureDefs.items(i).name(),
                featureDefs.items(i).type(),
                featureDefs.items(i).project(),
                featureDefs.items(i).enabled());
        defs.add(featureDef);
      }

      return defs;
    } catch (RuntimeException e) {
      log.warn("Unable to list known toggles, will return empty list", e);
      return new ArrayList<>();
    }
  }

  public WasmResponse<Boolean> isEnabled(String toggleName, Context context)
      throws YggdrasilInvalidInputException {
    try {
      Map<String, Boolean> strategyResults = customStrategiesEvaluator.eval(toggleName, context);
      byte[] contextBytes = buildMessage(toggleName, context, strategyResults);

      Response response = nativeInterface.checkEnabled(enginePointer, contextBytes);

      if (response.error() != null) {
        String error = response.error();
        throw new YggdrasilInvalidInputException(error);
      }

      if (response.hasEnabled()) {
        return new WasmResponse<Boolean>(response.impressionData(), response.enabled());
      } else {
        return new WasmResponse<Boolean>(response.impressionData(), null);
      }
    } catch (RuntimeException e) {
      log.warn("Could not check if toggle is enabled: {}", e.getMessage(), e);
      return new WasmResponse<Boolean>(false, null);
    }
  }

  public WasmResponse<VariantDef> getVariant(String toggleName, Context context)
      throws YggdrasilInvalidInputException {
    try {
      Map<String, Boolean> strategyResults = customStrategiesEvaluator.eval(toggleName, context);
      byte[] contextBytes = buildMessage(toggleName, context, strategyResults);

      Variant variant = nativeInterface.checkVariant(enginePointer, contextBytes);
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

        return new WasmResponse<VariantDef>(
            variant.impressionData(),
            new VariantDef(variant.name(), payload, variant.enabled(), variant.featureEnabled()));
      } else {
        return new WasmResponse<VariantDef>(false, null);
      }
    } catch (RuntimeException e) {
      log.warn("Could not get variant for toggle '{}': {}", toggleName, e.getMessage(), e);
      return new WasmResponse<VariantDef>(false, null);
    }
  }

  public MetricsBucket getMetrics() {
    try {
      ZonedDateTime now = ZonedDateTime.now(ZoneOffset.UTC);
      MetricsResponse response = UnleashFFI.getMetrics(this.enginePointer, now);
      if (response.togglesVector() == null) {
        return null;
      }

      Map<String, FeatureCount> toggles = new HashMap<>();
      for (int i = 0; i < response.togglesLength(); i++) {
        ToggleEntry toggleEntry = response.toggles(i);
        ToggleStats stats = toggleEntry.value();

        Map<String, Long> variants = new HashMap<>();
        for (int j = 0; j < stats.variantsLength(); j++) {
          VariantEntry variant = stats.variants(j);
          variants.put(variant.key(), variant.value());
        }
        FeatureCount featureCount = new FeatureCount(stats.yes(), stats.no(), variants);

        toggles.put(toggleEntry.key(), featureCount);
      }

      Instant startInstant = Instant.ofEpochMilli(response.start());
      Instant stopInstant = Instant.ofEpochMilli(response.stop());

      return new MetricsBucket(startInstant, stopInstant, toggles);
    } catch (RuntimeException e) {
      log.warn("Error retrieving metrics: {}", e.getMessage(), e);
      return null;
    }
  }

  static Pointer toUtf8Pointer(String str) {
    byte[] utf8Bytes = str.getBytes(StandardCharsets.UTF_8);
    Pointer pointer = new Memory(utf8Bytes.length + 1);
    pointer.write(0, utf8Bytes, 0, utf8Bytes.length);
    pointer.setByte(utf8Bytes.length, (byte) 0);
    return pointer;
  }

  // The following two methods break our abstraction a little by calling the
  // UnleashFFI directly. rather than through the nativeInterface. However,
  // we really, really want them to be accessible without having to instantiate
  // an UnleashEngine and our interface abstraction here is primarily for testing
  public static String getCoreVersion() {
    Pointer versionPointer = UnleashFFI.getCoreVersion();
    return versionPointer.getString(0);
  }

  public static List<String> getBuiltInStrategies() {
    BuiltInStrategies builtInStrategiesMessage = UnleashFFI.getBuiltInStrategies();
    List<String> builtInStrategies = new ArrayList<>(builtInStrategiesMessage.valuesLength());
    for (int i = 0; i < builtInStrategiesMessage.valuesLength(); i++) {
      String strategyName = builtInStrategiesMessage.values(i);
      builtInStrategies.add(strategyName);
    }

    return builtInStrategies;
  }
}
