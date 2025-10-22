package io.getunleash.engine;

import java.util.Optional;

public class VariantDef {
  private final String name;
  private final Payload payload;
  private final Boolean enabled;
  private final Boolean featureEnabled;

  VariantDef(
       String name,
       Payload payload,
       Boolean enabled,
       Boolean featureEnabled) {
    this.name = name;
    this.payload = payload;
    this.enabled = enabled;
    this.featureEnabled = Optional.ofNullable(featureEnabled).orElse(false);
  }

  public String getName() {
    return name;
  }

  public Payload getPayload() {
    return payload;
  }

  public Boolean isEnabled() {
    return enabled;
  }

  public Boolean isFeatureEnabled() {
    return featureEnabled;
  }
}
