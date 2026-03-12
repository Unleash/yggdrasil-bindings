package io.getunleash.engine;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;
import java.util.Locale;
import java.util.Optional;
import java.util.function.BooleanSupplier;
import java.util.function.Predicate;

final class LibNames {
  private enum LibcVersion {
    Glibc,
    Musl
  }

  /**
   * System property used to override libc detection.
   *
   * <p>Supported values:
   *
   * <ul>
   *   <li>{@code "musl"} – force musl libc selection
   *   <li>{@code "glibc"} – force glibc selection
   *   <li>{@code "gnu"} - alias for glibc
   *   <li>{@code "gnu libc"} - alias for glibc
   * </ul>
   *
   * Any other value (including {@code null}) will fall back to automatic detection.
   */
  private static final String LIBC_PROPERTY = "io.getunleash.engine.libc";

  /**
   * Environment variable used to override libc detection.
   *
   * <p>Supported values are the same as for {@link #LIBC_PROPERTY}: {@code "musl"}, {@code "glibc"}
   * {@code "gnu"}, {@code "gnu libc"}. Any other value (including unset) will fall back to
   * automatic detection.
   */
  private static final String LIBC_ENV = "UNLEASH_ENGINE_LIBC";

  static String pickForCurrentOsArch() {
    return pickFor(
        System.getProperty("os.name"),
        System.getProperty("os.arch"),
        System.getProperty(LIBC_PROPERTY),
        System.getenv(LIBC_ENV),
        LibNames::fileExists,
        LibNames::mapsContainMusl);
  }

  static String pickFor(
      String osName,
      String archName,
      String libcProperty,
      String libcEnv,
      Predicate<String> fileExists,
      BooleanSupplier mapsContainMusl) {
    String os = osName.toLowerCase(Locale.ROOT);
    String arch = archName.toLowerCase(Locale.ROOT);
    if (os.contains("mac")) {
      if (arch.contains("aarch64") || arch.contains("arm64")) {
        return "libyggdrasilffi_arm64.dylib";
      }
      return "libyggdrasilffi_x86_64.dylib";
    } else if (os.contains("win")) {
      if (arch.contains("arm64")) {
        return "yggdrasilffi_arm64.dll";
      }
      if (arch.contains("64")) {
        return "yggdrasilffi_x86_64.dll";
      }
      return "yggdrasilffi_i686.dll";
    } else { // linux
      LibcVersion version = libcVersion(libcProperty, libcEnv, fileExists, mapsContainMusl);
      if (version == LibcVersion.Musl) {
        if (arch.contains("aarch64") || arch.contains("arm64")) {
          return "libyggdrasilffi_arm64-musl.so";
        }
        return "libyggdrasilffi_x86_64-musl.so";
      } else {
        if (arch.contains("aarch64") || arch.contains("arm64")) {
          return "libyggdrasilffi_arm64.so";
        }
        return "libyggdrasilffi_x86_64.so";
      }
    }
  }

  static LibcVersion libcVersion(
      String libcProperty,
      String libcEnv,
      Predicate<String> fileExists,
      BooleanSupplier mapsContainMusl) {
    Optional<LibcVersion> override = parseLibcOverride(libcProperty);
    if (override.isPresent()) {
      return override.get();
    }

    override = parseLibcOverride(libcEnv);
    if (override.isPresent()) {
      return override.get();
    }

    for (String path : MUSL_MARKER_PATHS) {
      if (fileExists.test(path)) {
        return LibcVersion.Musl;
      }
    }

    if (mapsContainMusl.getAsBoolean()) {
      return LibcVersion.Musl;
    }
    return LibcVersion.Glibc;
  }

  private static final String[] MUSL_MARKER_PATHS = {
    "/etc/alpine-release",
    "/lib/ld-musl-x86_64.so.1",
    "/lib/ld-musl-aarch64.so.1",
    "/lib/ld-musl.so.1",
    "/usr/lib/ld-musl-x86_64.so.1",
    "/usr/lib/ld-musl-aarch64.so.1",
    "/usr/lib/ld-musl.so.1"
  };

  private static Optional<LibcVersion> parseLibcOverride(String value) {
    if (value == null) {
      return Optional.empty();
    }

    String normalized = value.trim().toLowerCase(Locale.ROOT);
    switch (normalized) {
      case "": return Optional.empty();
      case "glibc":
      case "gnu":
      case "gnu libc":
        return Optional.of(LibcVersion.Glibc);
      case "musl":
        return Optional.of(LibcVersion.Musl);
    }
    return Optional.empty();
  }

  private static boolean fileExists(String path) {
    return new java.io.File(path).exists();
  }

  // Dynamic JVMs typically expose the loaded musl loader in /proc/self/maps.
  // Statically linked runtimes do not, so this is only a final fallback.
  private static boolean mapsContainMusl() {
    try (BufferedReader reader = new BufferedReader(new FileReader("/proc/self/maps"))) {
      String line;
      while ((line = reader.readLine()) != null) {
        if (line.contains("musl")) {
          return true;
        }
      }
    } catch (IOException e) {
      System.err.println(
          "Warning: Failed to read /proc/self/maps, assuming this is not a musl system: "
              + e.getMessage());
    }
    return false;
  }
}
