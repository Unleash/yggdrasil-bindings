package io.getunleash.engine;

public class NativeException extends Exception {
  public NativeException(String message) {
    super(message);
  }

  public NativeException(String message, Throwable cause) {
    super(message, cause);
  }
}
