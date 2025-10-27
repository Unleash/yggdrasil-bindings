package io.getunleash.engine;

import java.util.Optional;

/**
 * Represents a variant definition.
 */
public class VariantDef {
  private final String name;
  private final Payload payload;
  private final Boolean enabled;
  private final Boolean featureEnabled;

  VariantDef(String name, Payload payload, Boolean enabled, Boolean featureEnabled) {
    this.name = name;
    this.payload = payload;
    this.enabled = enabled;
    this.featureEnabled = Optional.ofNullable(featureEnabled).orElse(false);
  }

  /**
   * Returns the name of the variant.
   *
   * @return the name of the variant
   */
  public String getName() {
    return name;
  }

  /**
   * Returns the payload of the variant.
   *
   * @return the payload of the variant
   */
  public Payload getPayload() {
    return payload;
  }

  /**
   * Returns whether the variant is enabled.
   *
   * @return true if the variant is enabled, false otherwise
   */
  public Boolean isEnabled() {
    return enabled;
  }

  /**
   * Returns whether the feature that the variant belongs to is enabled.
   *
   * @return true if the feature that the variant belongs to is enabled, false otherwise
   */
  public Boolean isFeatureEnabled() {
    return featureEnabled;
  }
}
