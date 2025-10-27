package io.getunleash.engine;

/**
 * Represents a payload for a variant definition.
 */
public class Payload {

  private String type;
  private String value;

  public String getType() {
    return type;
  }

  public String getValue() {
    return value;
  }

  public void setType(String newType) {
    type = newType;
  }

  public void setValue(String newValue) {
    value = newValue;
  }
}
