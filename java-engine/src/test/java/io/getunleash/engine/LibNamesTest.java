package io.getunleash.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class LibNamesTest {

  @Test
  void picksMuslBinaryWhenLibcPropertyOverridesDetection() {
    String libName = LibNames.pickFor("Linux", "amd64", "musl", null, path -> false, () -> false);

    assertEquals("libyggdrasilffi_x86_64-musl.so", libName);
  }

  @Test
  void propertyOverrideWinsOverMuslMarkers() {
    String libName = LibNames.pickFor("Linux", "amd64", "glibc", null, path -> true, () -> true);

    assertEquals("libyggdrasilffi_x86_64.so", libName);
  }

  @Test
  void picksMuslBinaryWhenLibcEnvOverridesDetection() {
    String libName = LibNames.pickFor("Linux", "amd64", null, "musl", path -> false, () -> false);

    assertEquals("libyggdrasilffi_x86_64-musl.so", libName);
  }

  @Test
  void propertyOverrideWinsOverEnvOverride() {
    String libName =
        LibNames.pickFor("Linux", "amd64", "glibc", "musl", path -> false, () -> false);

    assertEquals("libyggdrasilffi_x86_64.so", libName);
  }

  @Test
  void propertyOverrideWorksWithAliases() {
    String libcProperty = LibNames.pickFor("Linux", "amd64", "gnu", null, path -> true, () -> true);
    assertEquals("libyggdrasilffi_x86_64.so", libcProperty);
    String libcEnv = LibNames.pickFor("Linux", "amd64", null, "gnu", path -> true, () -> true);
    assertEquals("libyggdrasilffi_x86_64.so", libcEnv);
  }

  @Test
  void gnuLibcIsAlsoAValidAlias() {
    String libcProperty =
        LibNames.pickFor("Linux", "amd64", "gnu libc", null, path -> true, () -> true);
    assertEquals("libyggdrasilffi_x86_64.so", libcProperty);
    String libcEnv = LibNames.pickFor("Linux", "amd64", null, "gnu libc", path -> true, () -> true);
    assertEquals("libyggdrasilffi_x86_64.so", libcEnv);
  }

  @Test
  void picksMuslBinaryWhenAlpineMarkerExists() {
    String libName =
        LibNames.pickFor(
            "Linux",
            "aarch64",
            null,
            null,
            path -> path.equals("/etc/alpine-release"),
            () -> false);

    assertEquals("libyggdrasilffi_arm64-musl.so", libName);
  }

  @Test
  void fallsBackToMapsDetectionWhenNoFilesystemMarkerExists() {
    String libName = LibNames.pickFor("Linux", "amd64", null, null, path -> false, () -> true);

    assertEquals("libyggdrasilffi_x86_64-musl.so", libName);
  }
}
