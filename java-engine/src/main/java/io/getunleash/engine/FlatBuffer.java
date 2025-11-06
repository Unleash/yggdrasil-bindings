package io.getunleash.engine;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public class FlatBuffer {
  /**
   * Copies the direct byte buffer to heap, and handles freeing on the rust side, so don't call
   * {@link NativeBridge#flatBufFree} after calling this.
   *
   * @param nativeBuf the result from NativeBridge
   * @return a heap allocated ByteBuffer that can safely be used as root for FlatBuffer
   *     deserialization
   */
  static ByteBuffer toHeap(ByteBuffer nativeBuf) {
    ByteBuffer dup = nativeBuf.duplicate().order(ByteOrder.LITTLE_ENDIAN);
    byte[] arr = new byte[dup.remaining()];
    dup.get(arr);
    NativeBridge.flatBufFree(nativeBuf); // free native memory now
    return ByteBuffer.wrap(arr).order(ByteOrder.LITTLE_ENDIAN);
  }
}
