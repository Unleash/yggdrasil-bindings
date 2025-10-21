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

  UnleashFFI.Buf.ByValue checkEnabled(Pointer enginePointer, Pointer contextMessage);

  UnleashFFI.Buf.ByValue checkVariant(Pointer enginePointer, Pointer contextMessage);

  UnleashFFI.Buf.ByValue getMetrics(Pointer enginePointer, long timestamp);

  UnleashFFI.Buf.ByValue listKnownToggles(Pointer enginePointer);

  UnleashFFI.Buf.ByValue getBuiltInStrategies(Pointer enginePointer);

  UnleashFFI.Buf.ByValue getCoreVersion(Pointer enginePointer);

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

  void free_buf(Buf.ByValue buf);
}
