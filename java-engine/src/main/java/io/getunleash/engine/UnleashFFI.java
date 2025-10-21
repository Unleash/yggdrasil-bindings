package io.getunleash.engine;

import com.sun.jna.Library;
import com.sun.jna.NativeLong;
import com.sun.jna.Pointer;
import com.sun.jna.Structure;

import java.util.List;

public interface UnleashFFI extends Library {

  Pointer newEngine();

  void freeEngine(Pointer ptr);

  Pointer takeState(Pointer ptr, Pointer toggles);

  Pointer checkEnabled(Pointer ptr, Pointer name, Pointer context, Pointer customStrategyResults);

  Pointer checkVariant(Pointer ptr, Pointer name, Pointer context, Pointer customStrategyResults);

  Pointer countToggle(Pointer ptr, Pointer name, boolean enabled);

  Pointer countVariant(Pointer ptr, Pointer name, Pointer variantName);

  Pointer getMetrics(Pointer ptr);

  Pointer shouldEmitImpressionEvent(Pointer ptr, Pointer name);

  Pointer builtInStrategies();

  void freeResponse(Pointer pointer);

  Pointer listKnownToggles(Pointer ptr);

  static UnleashFFI getInstance() {
    return NativeLoader.NATIVE_INTERFACE;
  }

  static Pointer getYggdrasilCoreVersion() {
    return NativeLoader.NATIVE_INTERFACE.getCoreVersion();
  }

  Pointer getState(Pointer enginePointer);

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

    Buf.ByValue get_buf();
    void free_buf(Buf.ByValue buf);
}