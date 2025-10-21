package io.getunleash.engine;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

public class ResourceReader {
  public static String readResourceAsString(String resourceName) throws IOException {
    var resourceUrl = ResourceReader.class.getClassLoader().getResource(resourceName);
    if (resourceUrl == null) {
      throw new IllegalArgumentException("Resource not found: " + resourceName);
    }
    return Files.readString(Paths.get(resourceUrl.getPath()));
  }
}
