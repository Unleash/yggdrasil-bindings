package io.getunleash.engine;

import com.sun.jna.Pointer;
import messaging.FeatureDefs;
import messaging.MetricsResponse;
import messaging.Response;
import messaging.Variant;

import java.time.ZonedDateTime;

public interface NativeInterface {
    Pointer newEngine(long timestamp);
    void freeEngine(Pointer pointer);
    void takeState(Pointer pointer, byte[] messageBytes);
    String getState(Pointer pointer);
    Response checkEnabled(Pointer enginePointer, byte[] contextBytes);
    Variant checkVariant(Pointer enginePointer, byte[] contextBytes);
    MetricsResponse getMetrics(Pointer enginePointer, ZonedDateTime timestamp);
    Pointer getLogBufferPointer();
    FeatureDefs listKnownToggles(Pointer enginePointer);
}
