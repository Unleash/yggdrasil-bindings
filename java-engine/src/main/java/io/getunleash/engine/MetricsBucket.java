package io.getunleash.engine;

import java.time.Instant;
import java.util.Map;

public class MetricsBucket {
  private final Instant start;
  private final Instant stop;
  private final Map<String, FeatureCount> toggles;

  public MetricsBucket(Instant start, Instant stop, Map<String, FeatureCount> toggles) {
    this.start = start;
    this.stop = stop;
    this.toggles = toggles;
  }

  public Instant getStart() {
    return start;
  }

  public Instant getStop() {
    return stop;
  }

  public Map<String, FeatureCount> getToggles() {
    return toggles;
  }
}
