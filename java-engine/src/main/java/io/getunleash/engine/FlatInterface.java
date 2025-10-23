package io.getunleash.engine;

import com.sun.jna.Memory;
import com.sun.jna.Pointer;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.util.Map;

import io.getunleash.messaging.*;

public class FlatInterface implements NativeInterface {
  private final UnleashFFI unleashFFI;
  private final Pointer enginePointer;

  public FlatInterface(UnleashFFI unleashFFI) {
    this.unleashFFI = unleashFFI;
    this.enginePointer = unleashFFI.newEngine(Instant.now().toEpochMilli());
  }

  @Override
  public void freeEngine() {
    this.unleashFFI.freeEngine(enginePointer);
  }

  @Override
  public TakeStateResponse takeState(String toggles) {
    UnleashFFI.Buf.ByValue buf = this.unleashFFI.flatTakeState(enginePointer, toUtf8Pointer(toggles));
      long len = buf.len.longValue();
      byte[] out = buf.ptr.getByteArray(0, (int) len);
      this.unleashFFI.flatBufFree(buf);
      return TakeStateResponse.getRootAsTakeStateResponse(ByteBuffer.wrap(out));
  }

  @Override
  public String getState() {
    Pointer p = this.unleashFFI.getState(enginePointer);
    return p.getString(0);
  }

  @Override
  public Response checkEnabled(ByteBuffer contextMessage) {
    UnleashFFI.Buf.ByValue buf =
        this.unleashFFI.flatCheckEnabled(
            this.enginePointer, contextMessage, contextMessage.remaining());
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.flatBufFree(buf);
    return Response.getRootAsResponse(ByteBuffer.wrap(out));
  }

  @Override
  public Variant checkVariant(ByteBuffer contextMessage) {
    var buf =
        this.unleashFFI.flatCheckVariant(
            this.enginePointer, contextMessage, contextMessage.remaining());
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.flatBufFree(buf);
    return Variant.getRootAsVariant(ByteBuffer.wrap(out));
  }

  @Override
  public MetricsResponse getMetrics() {
    var buf = this.unleashFFI.flatGetMetrics(this.enginePointer);
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.flatBufFree(buf);
    return MetricsResponse.getRootAsMetricsResponse(ByteBuffer.wrap(out));
  }

  @Override
  public Pointer getLogBufferPointer() {
    return null;
  }

  @Override
  public FeatureDefs listKnownToggles() {
    var buf = this.unleashFFI.flatListKnownToggles(this.enginePointer);
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.flatBufFree(buf);
    return FeatureDefs.getRootAsFeatureDefs(ByteBuffer.wrap(out));
  }

  public static BuiltInStrategies getBuiltInStrategies() {
    UnleashFFI.Buf.ByValue buf = UnleashFFI.getInstance().flatBuiltInStrategies();
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    UnleashFFI.getInstance().flatBufFree(buf);
    return BuiltInStrategies.getRootAsBuiltInStrategies(ByteBuffer.wrap(out));
  }

  static Pointer toUtf8Pointer(String str) {
    byte[] utf8Bytes = str.getBytes(StandardCharsets.UTF_8);
    Pointer pointer = new Memory(utf8Bytes.length + 1);
    pointer.write(0, utf8Bytes, 0, utf8Bytes.length);
    pointer.setByte(utf8Bytes.length, (byte) 0);
    return pointer;
  }
}
