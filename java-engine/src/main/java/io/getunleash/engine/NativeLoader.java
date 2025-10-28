package io.getunleash.engine;

import java.io.*;

final class NativeLoader {

  static void loadFromResources(String libName) {
    try (var in = NativeLoader.class.getResourceAsStream("/native/" + libName)) {
      if (in == null) throw new IllegalStateException("Missing /native/" + libName);
      var tmp = java.nio.file.Files.createTempFile("ygg_", "_" + libName);
      tmp.toFile().deleteOnExit();
      try (var out = java.nio.file.Files.newOutputStream(tmp)) {
        in.transferTo(out);
      }
      System.load(tmp.toAbsolutePath().toString());
    } catch (Exception e) {
      throw new RuntimeException("Failed to load native lib " + libName, e);
    }
  }
}
