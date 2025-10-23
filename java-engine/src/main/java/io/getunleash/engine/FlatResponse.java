package io.getunleash.engine;

public class FlatResponse<T> {
  public boolean impressionData;

  public T value;

  public FlatResponse(boolean impressionData, T value) {
    this.impressionData = impressionData;
    this.value = value;
  }
}
