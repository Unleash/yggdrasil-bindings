package io.getunleash.engine;

import com.sun.jna.Library;
import com.sun.jna.NativeLong;
import com.sun.jna.Pointer;
import com.sun.jna.Structure;
import java.nio.ByteBuffer;
import java.util.List;

public interface UnleashFFI extends Library {

  /**
   * Creates a new engine instance with the given timestamp.
   *
   * <p>Must be freed using the {@link UnleashFFI#freeEngine(Pointer)} method.
   *
   * @param timestamp The timestamp to use for the engine.
   * @return A pointer to the new engine instance.
   */
  Pointer newEngine(long timestamp);

  /**
   * Frees the memory allocated for the engine instance.
   *
   * @param enginePointer The pointer to the engine instance to be freed.
   */
  void freeEngine(Pointer enginePointer);

  /**
   * Updates the state of the engine instance. Used when getting new toggles from upstream.
   *
   * @param enginePointer The pointer to the engine instance.
   * @param toggles A String pointer to the /api/client/features response.
   * @return A pointer to the updated engine instance state.
   */
  UnleashFFI.Buf.ByValue flatTakeState(Pointer enginePointer, Pointer toggles);

  /**
   * Retrieves the state of the engine instance as a String.
   *
   * @param enginePointer The pointer to the engine insance.
   * @return A String pointer to the state of the engine instance representation.
   */
  Pointer getState(Pointer enginePointer);

  /**
   * Checks if a feature toggle is enabled based on the provided context and the currently loaded
   * state of the engine.
   *
   * <p>Free after use, using {@link UnleashFFI#flatBufFree(UnleashFFI.Buf.ByValue)}.
   *
   * @param enginePointer The pointer to the engine instance.
   * @param contextMessage The context message to be used for the check.
   * @param bufferLength The length of the buffer.
   * @return A pointer to the result of the evaluation.
   */
  UnleashFFI.Buf.ByValue flatCheckEnabled(
      Pointer enginePointer, ByteBuffer contextMessage, long bufferLength);

  /**
   * Returns the active variant based on the provided context and the currently loaded state of the
   * engine.
   *
   * <p>Free after use, using {@link UnleashFFI#flatBufFree(UnleashFFI.Buf.ByValue)}.
   *
   * @param enginePointer The pointer to the engine instance.
   * @param contextMessage The context message to be used for the check.
   * @param bufferLength The length of the buffer.
   * @return A pointer to the result of the evaluation. Can be read, and should be freed after use.
   */
  UnleashFFI.Buf.ByValue flatCheckVariant(
      Pointer enginePointer, ByteBuffer contextMessage, long bufferLength);

  /**
   * Returns a list of known toggles based on the provided context and the currently loaded state of
   * the engine.
   *
   * <p>Free after use, using {@link UnleashFFI#flatBufFree(UnleashFFI.Buf.ByValue)}.
   *
   * @param enginePointer The pointer to the engine instance.
   * @return A pointer to the result of the evaluation.
   */
  UnleashFFI.Buf.ByValue flatListKnownToggles(Pointer enginePointer);

  /**
   * Returns a list of built-in strategies based on the provided context and the currently loaded
   * state of the engine.
   *
   * <p>Free after use, using {@link UnleashFFI#flatBufFree(UnleashFFI.Buf.ByValue)}.
   *
   * @return A pointer to the result of the evaluation.
   */
  UnleashFFI.Buf.ByValue flatBuiltInStrategies();

  /**
   * Returns current metrics bucket Free after use, using {@link
   * UnleashFFI#flatBufFree(UnleashFFI.Buf.ByValue)}.
   *
   * @param enginePointer The pointer to the engine instance.
   * @return A pointer to the result of the evaluation.
   */
  UnleashFFI.Buf.ByValue flatGetMetrics(Pointer enginePointer);

  /**
   * Returns a static pointer to the core version of the engine.
   *
   * <p>Do not free after use. Will cause Undefined Behavior.
   *
   * @return A string pointer to the yggdrasil version.
   */
  Pointer getCoreVersion();

  /** We should only load this once, so we cache the instance. */
  static UnleashFFI getInstance() {
    return NativeLoader.NATIVE_INTERFACE;
  }

  // struct by value
  class Buf extends Structure {
    /** A pointer to the data from the rust engine */
    public Pointer ptr;

    /** How many bytes are in the data from the engine */
    public NativeLong len;

    /** How many bytes are allocated for the data from the engine */
    public NativeLong cap;

    @Override
    protected List<String> getFieldOrder() {
      return java.util.List.of("ptr", "len", "cap");
    }

    /**
     * Used for the C ABI for passing a pointer to memory address plus length of data as well as the
     * capacity
     */
    public static class ByValue extends Buf implements Structure.ByValue {}
  }

  /**
   * Frees a buffer allocated by the engine.
   *
   * @param buf The buffer to free.
   */
  void flatBufFree(Buf.ByValue buf);
}
