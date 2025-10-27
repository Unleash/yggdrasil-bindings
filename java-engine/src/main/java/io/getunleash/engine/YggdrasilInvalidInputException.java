package io.getunleash.engine;

/** Used when we receive invalid input to Yggdrasil */
public class YggdrasilInvalidInputException extends Exception {
  /**
   * An instance of invalid input where we Yggdrasil could not understand the String we provided.
   *
   * @param input The input that was invalid
   */
  public YggdrasilInvalidInputException(String input) {
    super("The input provided is invalid: " + input);
  }

  /**
   * An instance of invalid input where we Yggdrasil could not understand the String we provided.
   * But where we had a sub Exception
   *
   * @param input The input that was invalid
   * @param cause The cause of the invalid input
   */
  public YggdrasilInvalidInputException(String input, Throwable cause) {
    super("The input provided is invalid: " + input, cause);
  }

  /**
   * An instance of invalid input where we Yggdrasil could not understand the Context we provided.
   *
   * @param input The context that was invalid
   */
  public YggdrasilInvalidInputException(Context input) {
    super("The context provided is invalid: " + input);
  }
}
