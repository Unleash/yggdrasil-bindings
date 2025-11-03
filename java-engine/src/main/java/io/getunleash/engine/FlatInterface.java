package io.getunleash.engine;

import com.google.flatbuffers.FlatBufferBuilder;
import io.getunleash.messaging.*;
import java.nio.ByteBuffer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class FlatInterface implements NativeInterface {
  private static final Logger LOGGER = LoggerFactory.getLogger(FlatInterface.class);

  private final long enginePointer;

  public FlatInterface() {
    this.enginePointer = NativeBridge.newEngine();
  }

  @Override
  public void freeEngine() {
    NativeBridge.freeEngine(enginePointer);
  }

  @Override
  public TakeStateResponse takeState(String toggles) {
    try {
      ByteBuffer result = NativeBridge.flatTakeState(enginePointer, toggles);
      return TakeStateResponse.getRootAsTakeStateResponse(FlatBuffer.toHeap(result));
    } catch (NativeException e) {
      LOGGER.warn(e.getMessage());
      return null;
    }
  }

  @Override
  public String getState() {
    try {
      return NativeBridge.flatGetState(enginePointer);
    } catch (NativeException e) {
      LOGGER.warn("Failed to get state. Our engine said {}", e.getMessage());
      return "{}";
    }
  }

  Response enabledResponse(String error) {
    FlatBufferBuilder flatBufferBuilder = new FlatBufferBuilder(16);
    int errorOffset = flatBufferBuilder.createString(error);
    Response.startResponse(flatBufferBuilder);
    Response.addError(flatBufferBuilder, errorOffset);
    Response.addHasEnabled(flatBufferBuilder, false);
    Response.addEnabled(flatBufferBuilder, false);
    Response.addImpressionData(flatBufferBuilder, false);
    Response.endResponse(flatBufferBuilder);
    return Response.getRootAsResponse(flatBufferBuilder.dataBuffer());
  }

  @Override
  public Response checkEnabled(ByteBuffer contextMessage) {
    try {
      ByteBuffer result =
          NativeBridge.flatCheckEnabled(enginePointer, contextMessage, contextMessage.remaining());
      return Response.getRootAsResponse(FlatBuffer.toHeap(result));
    } catch (NativeException e) {
      LOGGER.warn(
          "Our native engine failed to evaluate the context. The engine said {}. Returning default response (enabled: false, impressionData: false, hasEnabled: true)",
          e.getMessage());
      return enabledResponse(e.getMessage());
    }
  }

  Variant disabledVariant(String errorMessage) {
    FlatBufferBuilder flatBufferBuilder = new FlatBufferBuilder(16);
    int errorOffset = flatBufferBuilder.createString(errorMessage);
    int nameOffset = flatBufferBuilder.createString("disabled");
    Variant.startVariant(flatBufferBuilder);
    Variant.addEnabled(flatBufferBuilder, false);
    Variant.addError(flatBufferBuilder, errorOffset);
    Variant.addFeatureEnabled(flatBufferBuilder, false);
    Variant.addImpressionData(flatBufferBuilder, false);
    Variant.addName(flatBufferBuilder, nameOffset);
    Variant.endVariant(flatBufferBuilder);
    return Variant.getRootAsVariant(flatBufferBuilder.dataBuffer());
  }

  @Override
  public Variant checkVariant(ByteBuffer contextMessage) {
    try {
      ByteBuffer result =
          NativeBridge.flatCheckVariant(enginePointer, contextMessage, contextMessage.remaining());
      return Variant.getRootAsVariant(FlatBuffer.toHeap(result));
    } catch (NativeException e) {
      LOGGER.warn(
          "Our native engine failed to get variant from the passed in context. The engine said {}. Returning default variant",
          e.getMessage());
      return disabledVariant(e.getMessage());
    }
  }

  static MetricsResponse emptyMetrics() {
    FlatBufferBuilder builder = new FlatBufferBuilder(8);
    MetricsResponse.startMetricsResponse(builder);
    MetricsResponse.endMetricsResponse(builder);
    return MetricsResponse.getRootAsMetricsResponse(builder.dataBuffer());
  }

  @Override
  public MetricsResponse getMetrics() {
    try {
      ByteBuffer result = NativeBridge.flatGetMetrics(enginePointer);
      return MetricsResponse.getRootAsMetricsResponse(FlatBuffer.toHeap(result));
    } catch (NativeException e) {
      LOGGER.warn(
          "Our native engine failed to get metrics. The error was [{}]. Returning an empty metrics response",
          e.getMessage());
      return FlatInterface.emptyMetrics();
    }
  }

  static FeatureDefs emptyFeatureDef() {
    FlatBufferBuilder builder = new FlatBufferBuilder(8);
    FeatureDefs.startFeatureDefs(builder);
    FeatureDefs.endFeatureDefs(builder);
    return FeatureDefs.getRootAsFeatureDefs(builder.dataBuffer());
  }

  @Override
  public FeatureDefs listKnownToggles() {
    try {
      ByteBuffer buf = NativeBridge.flatListKnownToggles(enginePointer);
      return FeatureDefs.getRootAsFeatureDefs(FlatBuffer.toHeap(buf));
    } catch (NativeException e) {
      LOGGER.warn(
          "Could get known toggles from the native engine. The error was [{}]", e.getMessage());
      return FlatInterface.emptyFeatureDef();
    }
  }

  static BuiltInStrategies emptyBuiltInStrategies() {
    FlatBufferBuilder builder = new FlatBufferBuilder(8);
    BuiltInStrategies.startBuiltInStrategies(builder);
    BuiltInStrategies.endBuiltInStrategies(builder);
    return BuiltInStrategies.getRootAsBuiltInStrategies(builder.dataBuffer());
  }

  public static BuiltInStrategies getBuiltInStrategies() {
    try {
      ByteBuffer byteBuffer = NativeBridge.flatBuiltInStrategies();
      return BuiltInStrategies.getRootAsBuiltInStrategies(FlatBuffer.toHeap(byteBuffer));
    } catch (NativeException e) {
      LOGGER.warn(
          "Could not get built in strategies the native engine. This signifies a complete breakdown of the native code. Please let Unleash know. The error was [{}]",
          e.getMessage());
      return FlatInterface.emptyBuiltInStrategies();
    }
  }
}
