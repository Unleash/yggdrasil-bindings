package io.getunleash.engine;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;
import java.util.Locale;
import java.util.function.BooleanSupplier;
import java.util.function.Predicate;

final class LibNames {
  static final String LIBC_PROPERTY = "io.getunleash.engine.libc";
  static final String LIBC_ENV = "UNLEASH_ENGINE_LIBC";

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
      if (isMusl(libcProperty, libcEnv, fileExists, mapsContainMusl)) {
        if (arch.contains("aarch64") || arch.contains("arm64")) {
          return "libyggdrasilffi_arm64-musl.so";
        }
        return "libyggdrasilffi_x86_64-musl.so";
      }
      if (arch.contains("aarch64") || arch.contains("arm64")) {
        return "libyggdrasilffi_arm64.so";
      }
      return "libyggdrasilffi_x86_64.so";
    }
  }

  static boolean isMusl(
      String libcProperty,
      String libcEnv,
      Predicate<String> fileExists,
      BooleanSupplier mapsContainMusl) {
    Boolean override = parseLibcOverride(libcProperty);
    if (override != null) {
      return override;
    }

    override = parseLibcOverride(libcEnv);
    if (override != null) {
      return override;
    }

    for (String path : MUSL_MARKER_PATHS) {
      if (fileExists.test(path)) {
        return true;
      }
    }

    return mapsContainMusl.getAsBoolean();
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

  private static Boolean parseLibcOverride(String value) {
    if (value == null) {
      return null;
    }

    String normalized = value.trim().toLowerCase(Locale.ROOT);
    if (normalized.isEmpty()) {
      return null;
    }

    if (normalized.equals("musl")) {
      return true;
    }

    if (normalized.equals("glibc") || normalized.equals("gnu") || normalized.equals("gnu libc")) {
      return false;
    }

    System.err.println(
        "Warning: Unsupported libc override '"
            + value
            + "'. Expected 'musl' or 'glibc' (aliases: 'gnu', 'gnu libc'). Ignoring.");
    return null;
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
