package io.getunleash.engine;

import java.util.Map;

public class FeatureCount {
  private final Long yes;
  private final Long no;
  private final Map<String, Long> variants;

  public FeatureCount(Long yes, Long no, Map<String, Long> variants) {
    this.yes = yes;
    this.no = no;
    this.variants = variants;
  }

  public Long getYes() {
    return yes;
  }

  public Long getNo() {
    return no;
  }

  public Map<String, Long> getVariants() {
    return variants;
  }
}
