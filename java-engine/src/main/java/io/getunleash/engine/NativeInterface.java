package io.getunleash.engine;

import com.sun.jna.Pointer;
import java.time.ZonedDateTime;
import messaging.*;

public interface NativeInterface {
  void freeEngine();

  void takeState(String toggles);

  String getState();

  Response checkEnabled(byte[] contextMessage);

  Variant checkVariant(byte[] contextMessage);

  MetricsResponse getMetrics(ZonedDateTime timestamp);

  Pointer getLogBufferPointer();

  FeatureDefs listKnownToggles();

  BuiltInStrategies getBuiltInStrategies();

  String getCoreVersion();
}
