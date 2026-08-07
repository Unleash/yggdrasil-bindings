package io.getunleash.engine;

import java.io.*;

final class NativeLoader {

  static void loadFromResources(LibNames.NativeLibrary library) {
    try (var in = NativeLoader.class.getResourceAsStream(library.resourcePath())) {
      if (in == null) throw new IllegalStateException("Missing " + library.resourcePath());
      var tmp = java.nio.file.Files.createTempFile("ygg_", "_" + library.fileName());
      tmp.toFile().deleteOnExit();
      try (var out = java.nio.file.Files.newOutputStream(tmp)) {
        in.transferTo(out);
      }
      System.load(tmp.toAbsolutePath().toString());
    } catch (Exception e) {
      throw new RuntimeException("Failed to load native lib " + library.fileName(), e);
    }
  }
}
