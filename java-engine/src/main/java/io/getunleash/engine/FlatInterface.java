package io.getunleash.engine;

import com.sun.jna.Memory;
import com.sun.jna.Pointer;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.time.ZonedDateTime;
import messaging.*;

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
  public void takeState(String toggles) {
    this.unleashFFI.takeState(enginePointer, toUtf8Pointer(toggles));
  }

  @Override
  public String getState() {
    Pointer p = this.unleashFFI.getState(enginePointer);
    return p.getString(0);
  }

  @Override
  public Response checkEnabled(byte[] contextMessage) {
    UnleashFFI.Buf.ByValue buf =
        this.unleashFFI.checkEnabled(this.enginePointer, toPointer(contextMessage));
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.free_buf(buf);
    return Response.getRootAsResponse(ByteBuffer.wrap(out));
  }

  @Override
  public Variant checkVariant(byte[] contextMessage) {
    var buf = this.unleashFFI.checkVariant(this.enginePointer, toPointer(contextMessage));
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.free_buf(buf);
    return Variant.getRootAsVariant(ByteBuffer.wrap(out));
  }

  @Override
  public MetricsResponse getMetrics(ZonedDateTime timestamp) {
    var buf = this.unleashFFI.getMetrics(this.enginePointer, timestamp.toInstant().toEpochMilli());
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.free_buf(buf);
    return MetricsResponse.getRootAsMetricsResponse(ByteBuffer.wrap(out));
  }

  @Override
  public Pointer getLogBufferPointer() {
    return null;
  }

  @Override
  public FeatureDefs listKnownToggles() {
    var buf = this.unleashFFI.listKnownToggles(this.enginePointer);
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.free_buf(buf);
    return FeatureDefs.getRootAsFeatureDefs(ByteBuffer.wrap(out));
  }

  @Override
  public String getCoreVersion() {
    UnleashFFI.Buf.ByValue coreVersion = this.unleashFFI.getCoreVersion(enginePointer);
    long len = coreVersion.len.longValue();
    byte[] out = coreVersion.ptr.getByteArray(0, (int) len);
    this.unleashFFI.free_buf(coreVersion);
    return CoreVersion.getRootAsCoreVersion(ByteBuffer.wrap(out)).version();
  }

  @Override
  public BuiltInStrategies getBuiltInStrategies() {
    UnleashFFI.Buf.ByValue buf = this.unleashFFI.getBuiltInStrategies(this.enginePointer);
    long len = buf.len.longValue();
    byte[] out = buf.ptr.getByteArray(0, (int) len);
    this.unleashFFI.free_buf(buf);
    return BuiltInStrategies.getRootAsBuiltInStrategies(ByteBuffer.wrap(out));
  }

  static Pointer toUtf8Pointer(String str) {
    byte[] utf8Bytes = str.getBytes(StandardCharsets.UTF_8);
    return toPointer(utf8Bytes);
  }

  static Pointer toPointer(byte[] bytes) {
    Pointer pointer = new Memory(bytes.length + 1);
    pointer.write(0, bytes, 0, bytes.length);
    pointer.setByte(bytes.length, (byte) 0);
    return pointer;
  }
}
