package io.getunleash.engine;

import static org.assertj.core.api.Assertions.assertThat;

import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class LibNamesTest {
  @ParameterizedTest
  @CsvSource({
    "aarch64, windows-arm64",
    "arm64, windows-arm64",
    "amd64, windows-x86_64",
    "x86_64, windows-x86_64",
    "x86, windows-i686"
  })
  void selectsWindowsLibraryForJvmArchitecture(String arch, String expectedDirectory) {
    var library = LibNames.pickForOsArch("Windows 11", arch, "0.20.8");

    assertThat(library.resourcePath())
        .isEqualTo("/native/" + expectedDirectory + "/yggdrasilffi-0.20.8.dll");
  }
}
