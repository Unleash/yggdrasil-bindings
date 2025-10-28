package io.getunleash.engine;

import io.getunleash.messaging.*;
import java.nio.ByteBuffer;

public class FlatInterface implements NativeInterface {
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
    ByteBuffer result = NativeBridge.flatTakeState(enginePointer, toggles);
    return TakeStateResponse.getRootAsTakeStateResponse(FlatBuffer.toHeap(result));
  }

  @Override
  public String getState() {
    return NativeBridge.flatGetState(enginePointer);
  }

  @Override
  public Response checkEnabled(ByteBuffer contextMessage) {
    ByteBuffer result =
        NativeBridge.flatCheckEnabled(enginePointer, contextMessage, contextMessage.remaining());
    return Response.getRootAsResponse(FlatBuffer.toHeap(result));
  }

  @Override
  public Variant checkVariant(ByteBuffer contextMessage) {
    ByteBuffer result =
        NativeBridge.flatCheckVariant(enginePointer, contextMessage, contextMessage.remaining());
    return Variant.getRootAsVariant(FlatBuffer.toHeap(result));
  }

  @Override
  public MetricsResponse getMetrics() {
    ByteBuffer result = NativeBridge.flatGetMetrics(enginePointer);
    return MetricsResponse.getRootAsMetricsResponse(FlatBuffer.toHeap(result));
  }

  @Override
  public FeatureDefs listKnownToggles() {
    ByteBuffer buf = NativeBridge.flatListKnownToggles(enginePointer);
    return FeatureDefs.getRootAsFeatureDefs(FlatBuffer.toHeap(buf));
  }

  public static BuiltInStrategies getBuiltInStrategies() {
    ByteBuffer byteBuffer = NativeBridge.flatBuiltInStrategies();
    return BuiltInStrategies.getRootAsBuiltInStrategies(FlatBuffer.toHeap(byteBuffer));
  }
}
