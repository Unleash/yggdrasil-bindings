package io.getunleash.engine;

import java.util.Optional;

public class FeatureDef {
  private final String name;
  private final Optional<String> type;
  private final String project;
  private final boolean enabled;

  FeatureDef(String name, String featureType, String project, boolean enabled) {
    this.name = name;
    this.project = project;
    this.type = Optional.ofNullable(featureType);
    this.enabled = enabled;
  }

  public String getName() {
    return name;
  }

  public Optional<String> getType() {
    return type;
  }

  public String getProject() {
    return project;
  }

  public boolean isEnabled() {
    return enabled;
  }
}
