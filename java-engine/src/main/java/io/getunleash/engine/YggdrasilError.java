package io.getunleash.engine;

/** Used when an error occurs while interacting with Yggdrasil. */
public class YggdrasilError extends Exception {
  /**
   * A generic error for Yggdrasil
   *
   * @param message The error message
   */
  public YggdrasilError(String message) {
    super(message);
  }

  /**
   * A generic error for Yggdrasil with a cause
   *
   * @param message The error message
   * @param cause The cause of the error
   */
  public YggdrasilError(String message, Throwable cause) {
    super(message, cause);
  }
}
