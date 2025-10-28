package io.getunleash.engine;

import java.nio.ByteBuffer;

public final class NativeBridge {
  static {
    NativeLoader.loadFromResources(LibNames.pickForCurrentOsArch());
  }

  // Engine lifecycle
  public static native long newEngine(); // NOTE: no args (matches your C ABI)

  public static native void freeEngine(long ptr);

  // State update (toggles JSON goes in, serialized flatbuf ByteBuffer comes out)
  public static native ByteBuffer flatTakeState(long enginePtr, String togglesJson);

  public static native String flatGetState(long enginePtr);

  // Queries (input ctx is a direct buffer; result is a direct buffer you must free)
  public static native ByteBuffer flatCheckEnabled(long enginePtr, ByteBuffer ctx, long len);

  public static native ByteBuffer flatCheckVariant(long enginePtr, ByteBuffer ctx, long len);

  public static native ByteBuffer flatListKnownToggles(long enginePtr);

  public static native ByteBuffer flatBuiltInStrategies();

  public static native ByteBuffer flatGetMetrics(long enginePtr);

  public static native String getCoreVersion();

  // Free any ByteBuffer returned by the methods above
  public static native void flatBufFree(ByteBuffer buf);
}
