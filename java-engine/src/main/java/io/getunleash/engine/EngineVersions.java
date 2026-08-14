package io.getunleash.engine;

import java.io.IOException;
import java.util.Properties;

final class EngineVersions {
  private static final String VERSION_RESOURCE = "/io/getunleash/engine/version.properties";
  private static final Properties PROPERTIES = loadProperties();

  private EngineVersions() {}

  static String getBundledYggdrasilCoreVersion() {
    return requireProperty("yggdrasilCoreVersion");
  }

  private static Properties loadProperties() {
    var properties = new Properties();
    try (var in = EngineVersions.class.getResourceAsStream(VERSION_RESOURCE)) {
      if (in == null) {
        throw new IllegalStateException("Missing " + VERSION_RESOURCE);
      }
      properties.load(in);
      return properties;
    } catch (IOException e) {
      throw new IllegalStateException("Failed to read " + VERSION_RESOURCE, e);
    }
  }

  private static String requireProperty(String propertyName) {
    var value = PROPERTIES.getProperty(propertyName);
    if (value == null || value.trim().isEmpty()) {
      throw new IllegalStateException("Missing " + propertyName + " in " + VERSION_RESOURCE);
    }
    return value.trim();
  }
}
