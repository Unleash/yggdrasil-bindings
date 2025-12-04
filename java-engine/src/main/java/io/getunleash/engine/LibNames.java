package io.getunleash.engine;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;

final class LibNames {
  static String pickForCurrentOsArch() {
    String os = System.getProperty("os.name").toLowerCase();
    String arch = System.getProperty("os.arch").toLowerCase();
    if (os.contains("mac")) {
      return arch.contains("aarch64") || arch.contains("arm64")
          ? "libyggdrasilffi_arm64.dylib"
          : "libyggdrasilffi_x86_64.dylib";
    } else if (os.contains("win")) {
      if (arch.contains("arm64")) return "yggdrasilffi_arm64.dll";
      if (arch.contains("64")) return "yggdrasilffi_x86_64.dll";
      return "yggdrasilffi_i686.dll";
    } else { // linux
      if (isMusl()) {
        return arch.contains("aarch64") || arch.contains("arm64")
            ? "libyggdrasilffi_arm64-musl.so"
            : "libyggdrasilffi_x86_64-musl.so";
      }
      return arch.contains("aarch64") || arch.contains("arm64")
          ? "libyggdrasilffi_arm64.so"
          : "libyggdrasilffi_x86_64.so";
    }
  }

  // Since System.getProperty("os.name") just lists 'linux'
  // But! Because the JVM itself is dynamically linked against either libc or
  // musl, and we know the JVM is currently running, it must have loaded ld-musl
  // into memory in order to run on a musl system, so we can just query the list
  // of loaded libraries and check if ld-musl is in there
  private static boolean isMusl() {
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
