package io.getunleash.engine;

import java.io.*;

final class NativeLoader {

  static void loadFromResources(LibNames.NativeLibrary library) {
    try {
      System.loadLibrary(library.libraryName());
      return;
    } catch (UnsatisfiedLinkError e) {
      // was a long shot anyway but it's polite to try to load from standard system
      // paths before we fallback to more invasive options
    }

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
}
