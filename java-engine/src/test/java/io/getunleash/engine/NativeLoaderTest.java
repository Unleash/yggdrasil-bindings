package io.getunleash.engine;

import static org.assertj.core.api.Assertions.assertThat;

import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class NativeLoaderTest {

  private static final LibNames.NativeLibrary NATIVE_LIBRARY =
      new LibNames.NativeLibrary(
          "linux-x86_64", "libyggdrasilffi-0.20.7.so", "yggdrasilffi-0.20.7");

  @Test
  void configuredLibraryPathUsesDirectFilePath() {
    var configuredPath = "/opt/unleash/libyggdrasilffi-0.20.7.so";

    assertThat(NativeLoader.configuredLibraryPath(configuredPath, NATIVE_LIBRARY))
        .isEqualTo(Path.of(configuredPath));
  }

  @Test
  void configuredLibraryPathResolvesVersionedFileNameInsideConfiguredDirectory(@TempDir Path dir) {
    assertThat(NativeLoader.configuredLibraryPath(dir.toString(), NATIVE_LIBRARY))
        .isEqualTo(dir.resolve("libyggdrasilffi-0.20.7.so"));
  }
}
