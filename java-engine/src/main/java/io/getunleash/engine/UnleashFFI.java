package io.getunleash.engine;

import com.sun.jna.Library;
import com.sun.jna.NativeLong;
import com.sun.jna.Pointer;
import com.sun.jna.Structure;
import java.util.List;

public interface UnleashFFI extends Library {

  Pointer newEngine(long timestamp);

  void freeEngine(Pointer enginePointer);

  void takeState(Pointer enginePointer, Pointer toggles);

  Pointer getState(Pointer enginePointer);

  UnleashFFI.Buf.ByValue flatCheckEnabled(Pointer enginePointer, Pointer contextMessage);

  UnleashFFI.Buf.ByValue flatCheckVariant(Pointer enginePointer, Pointer contextMessage);

  UnleashFFI.Buf.ByValue getMetrics(Pointer enginePointer, long timestamp);

  UnleashFFI.Buf.ByValue flatListKnownToggles(Pointer enginePointer);

  UnleashFFI.Buf.ByValue getBuiltInStrategies(Pointer enginePointer);

  Pointer getCoreVersion();

  static UnleashFFI getInstance() {
    return NativeLoader.NATIVE_INTERFACE;
  }

  // struct by value
  class Buf extends Structure {
    public Pointer ptr;
    public NativeLong len;
    public NativeLong cap;

    @Override
    protected List<String> getFieldOrder() {
      return java.util.List.of("ptr", "len", "cap");
    }

    public static class ByValue extends Buf implements Structure.ByValue {}
  }

  void flatBufFree(Buf.ByValue buf);
}
