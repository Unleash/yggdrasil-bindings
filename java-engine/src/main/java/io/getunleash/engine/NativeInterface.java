package io.getunleash.engine;

import com.sun.jna.Pointer;
import io.getunleash.messaging.*;
import java.nio.ByteBuffer;

public interface NativeInterface {
  void freeEngine();

  TakeStateResponse takeState(String toggles);

  String getState();

  Response checkEnabled(ByteBuffer contextMessage);

  Variant checkVariant(ByteBuffer contextMessage);

  MetricsResponse getMetrics();

  Pointer getLogBufferPointer();

  FeatureDefs listKnownToggles();

}
