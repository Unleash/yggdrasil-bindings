package io.getunleash.engine;

import com.sun.jna.Pointer;
import io.getunleash.messaging.*;
import java.nio.ByteBuffer;

/** Represents the native interface for interacting with the Unleash engine. */
public interface NativeInterface {
  /** Frees the memory allocated for the engine. */
  void freeEngine();

  /**
   * Loads state into the engine.
   *
   * @param toggles The state to load.
   */
  TakeStateResponse takeState(String toggles);

  /**
   * Retrieves the current state of the engine.
   *
   * @return The current state of the engine.
   */
  String getState();

  /**
   * Checks if a feature is enabled.
   *
   * @param contextMessage The context message to use for the check.
   * @return The result of the check.
   */
  Response checkEnabled(ByteBuffer contextMessage);

  /**
   * Get active variant for toggle
   *
   * @param contextMessage The context message to use for the check.
   * @return The active variant for the toggle.
   */
  Variant checkVariant(ByteBuffer contextMessage);

  /**
   * Get metrics for the engine.
   *
   * @return The metrics for the engine.
   */
  MetricsResponse getMetrics();

  Pointer getLogBufferPointer();

  /**
   * List known toggles.
   *
   * @return The list of known toggles.
   */
  FeatureDefs listKnownToggles();
}
