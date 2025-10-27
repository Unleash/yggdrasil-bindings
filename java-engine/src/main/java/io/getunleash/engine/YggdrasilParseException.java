package io.getunleash.engine;

/** Used when parsing a Yggdrasil response fails. */
public class YggdrasilParseException extends RuntimeException {
  /**
   * Constructor for YggdrasilParseException
   *
   * <p>Should include the input that was used as well as what we tried toV parse it into.
   *
   * @param <T> The class of the object we're trying to parse into.
   * @param input The input that was used
   * @param target The class we tried to parse into
   * @param parent The exception that caused this exception
   */
  public <T> YggdrasilParseException(String input, Class<T> target, Exception parent) {
    super("Can't read " + input + " into " + target, parent);
  }
}
