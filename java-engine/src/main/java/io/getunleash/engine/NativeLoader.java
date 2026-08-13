package io.getunleash.engine;

import java.io.*;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

final class NativeLoader {

  static void loadFromResources(LibNames.NativeLibrary library) {
    var extractedLibrary = extractedLibraryPath(library);
    try {
      if (Files.exists(extractedLibrary)) {
        try {
          System.load(extractedLibrary.toAbsolutePath().toString());
          return;
        } catch (UnsatisfiedLinkError e) {
          extractLibrary(library, extractedLibrary);
          System.load(extractedLibrary.toAbsolutePath().toString());
          return;
        }
      }

      extractLibrary(library, extractedLibrary);
      System.load(extractedLibrary.toAbsolutePath().toString());
    } catch (Exception | UnsatisfiedLinkError e) {
      throw new RuntimeException("Failed to load native lib " + library.resourcePath(), e);
    }
  }

  private static Path extractedLibraryPath(LibNames.NativeLibrary library) {
    return Path.of(
        System.getProperty("java.io.tmpdir"),
        "io.getunleash",
        "yggdrasil-engine",
        library.platformDirectory(),
        library.fileName());
  }

  private static void extractLibrary(LibNames.NativeLibrary library, Path destination)
      throws IOException {
    Files.createDirectories(destination.getParent());
    var tempFile = Files.createTempFile(destination.getParent(), library.fileName(), ".tmp");
    try (var in = NativeLoader.class.getResourceAsStream(library.resourcePath())) {
      if (in == null) throw new IllegalStateException("Missing " + library.resourcePath());
      try (var out = Files.newOutputStream(tempFile)) {
        in.transferTo(out);
      }
      moveIntoPlace(tempFile, destination);
    } finally {
      Files.deleteIfExists(tempFile);
    }
  }

  private static void moveIntoPlace(Path source, Path destination) throws IOException {
    try {
      try {
        Files.move(
            source,
            destination,
            StandardCopyOption.ATOMIC_MOVE,
            StandardCopyOption.REPLACE_EXISTING);
      } catch (AtomicMoveNotSupportedException e) {
        Files.move(source, destination, StandardCopyOption.REPLACE_EXISTING);
      }
    } catch (IOException e) {
      if (Files.exists(destination)) {
        return;
      }
      throw e;
    }
  }
}
