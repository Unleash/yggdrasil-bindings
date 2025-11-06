package io.getunleash.engine;

import java.nio.ByteBuffer;

public final class NativeBridge {
  static {
    NativeLoader.loadFromResources(LibNames.pickForCurrentOsArch());
  }

  // Engine lifecycle
  public static native long newEngine();

  public static native void freeEngine(long ptr);

  // State update (toggles JSON goes in, result is a direct buffer you must free)
  public static native ByteBuffer flatTakeState(long enginePtr, String togglesJson)
      throws NativeException;

  public static native String flatGetState(long enginePtr) throws NativeException;

  // Queries (input ctx is a direct buffer; result is a direct buffer you must free)
  public static native ByteBuffer flatCheckEnabled(long enginePtr, ByteBuffer ctx, long len)
      throws NativeException;

  public static native ByteBuffer flatCheckVariant(long enginePtr, ByteBuffer ctx, long len)
      throws NativeException;

  public static native ByteBuffer flatListKnownToggles(long enginePtr) throws NativeException;

  public static native ByteBuffer flatBuiltInStrategies() throws NativeException;

  public static native ByteBuffer flatGetMetrics(long enginePtr) throws NativeException;

  public static native String getCoreVersion();

  // Free any ByteBuffer returned by the methods above
  public static native void flatBufFree(ByteBuffer buf);
}
