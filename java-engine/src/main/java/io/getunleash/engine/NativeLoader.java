package io.getunleash.engine;

import java.nio.file.Files;
import java.nio.file.Path;

final class NativeLoader {
  static final String NATIVE_LIBRARY_PATH_PROPERTY = "io.getunleash.engine.native.path";

  static void loadFromResources(LibNames.NativeLibrary library) {
    loadFromConfiguredPath(library);

    try (var in = NativeLoader.class.getResourceAsStream(library.resourcePath())) {
      if (in == null) throw new IllegalStateException("Missing " + library.resourcePath());
      var tmp = java.nio.file.Files.createTempFile("ygg_", "_" + library.fileName());
      tmp.toFile().deleteOnExit();
      try (var out = java.nio.file.Files.newOutputStream(tmp)) {
        in.transferTo(out);
      }
      System.load(tmp.toAbsolutePath().toString());
    } catch (Exception e) {
      throw new RuntimeException("Failed to load native lib " + library.resourcePath(), e);
    }
  }

  private static void loadFromConfiguredPath(LibNames.NativeLibrary library) {
    var configuredPath = System.getProperty(NATIVE_LIBRARY_PATH_PROPERTY);
    if (configuredPath == null || configuredPath.trim().isEmpty()) {
      return;
    }

    var libraryPath = configuredLibraryPath(configuredPath.trim(), library).toAbsolutePath();
    try {
      System.load(libraryPath.toString());
    } catch (UnsatisfiedLinkError e) {
      throw new RuntimeException(
          "Failed to load native lib from "
              + NATIVE_LIBRARY_PATH_PROPERTY
              + "="
              + configuredPath
              + " resolved to "
              + libraryPath,
          e);
    }
  }

  static Path configuredLibraryPath(String configuredPath, LibNames.NativeLibrary library) {
    var path = Path.of(configuredPath);
    return Files.isDirectory(path) ? path.resolve(library.fileName()) : path;
  }
}
