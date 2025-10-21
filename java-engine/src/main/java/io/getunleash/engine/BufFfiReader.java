package io.getunleash.engine;

public class BufFfiReader {
  public static byte[] getBytes() {
    UnleashFFI.Buf.ByValue b = UnleashFFI.getInstance().get_buf();
    long len = b.len.longValue();
    byte[] out = b.ptr.getByteArray(0, (int) len);
    UnleashFFI.getInstance().free_buf(b);
    return out;
  }
}
