package io.getunleash.engine;

import com.sun.jna.Pointer;
import java.time.ZonedDateTime;
import messaging.FeatureDefs;
import messaging.MetricsResponse;
import messaging.Response;
import messaging.Variant;

public class FlatInterface implements NativeInterface {

  private UnleashFFI ffi;
  private Pointer enginePointer;

  public FlatInterface(UnleashFFI ffi) {
    this.ffi = ffi;
    this.enginePointer = ffi.newEngine();
  }

  @Override
  public void takeState(Pointer pointer, byte[] messageBytes) {}

  @Override
  public String getState(Pointer pointer) {
    return "";
  }

  @Override
  public Response checkEnabled(Pointer enginePointer, byte[] contextBytes) {
    return null;
  }

  @Override
  public Variant checkVariant(Pointer enginePointer, byte[] contextBytes) {
    return null;
  }

  @Override
  public MetricsResponse getMetrics(Pointer enginePointer, ZonedDateTime timestamp) {
    return null;
  }

  @Override
  public Pointer getLogBufferPointer() {
    return null;
  }

  @Override
  public FeatureDefs listKnownToggles(Pointer enginePointer) {
    return null;
  }
}
