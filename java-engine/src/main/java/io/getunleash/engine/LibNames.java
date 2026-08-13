package io.getunleash.engine;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;
import java.util.Locale;

final class LibNames {
  static NativeLibrary pickForCurrentOsArch(String version) {
    String os = System.getProperty("os.name").toLowerCase(Locale.ROOT);
    String arch = System.getProperty("os.arch").toLowerCase(Locale.ROOT);
    String libraryName = "yggdrasilffi-" + version;
    if (os.contains("mac")) {
      return arch.contains("aarch64") || arch.contains("arm64")
          ? new NativeLibrary("macos-arm64", "libyggdrasilffi-" + version + ".dylib", libraryName)
          : new NativeLibrary("macos-x86_64", "libyggdrasilffi-" + version + ".dylib", libraryName);
    } else if (os.contains("win")) {
      if (arch.contains("arm64")) {
        return new NativeLibrary("windows-arm64", "yggdrasilffi-" + version + ".dll", libraryName);
      }
      if (arch.contains("64")) {
        return new NativeLibrary("windows-x86_64", "yggdrasilffi-" + version + ".dll", libraryName);
      }
      return new NativeLibrary("windows-i686", "yggdrasilffi-" + version + ".dll", libraryName);
    } else { // linux
      if (isMusl()) {
        return arch.contains("aarch64") || arch.contains("arm64")
            ? new NativeLibrary(
                "linux-arm64-musl", "libyggdrasilffi-" + version + ".so", libraryName)
            : new NativeLibrary(
                "linux-x86_64-musl", "libyggdrasilffi-" + version + ".so", libraryName);
      }
      return arch.contains("aarch64") || arch.contains("arm64")
          ? new NativeLibrary("linux-arm64", "libyggdrasilffi-" + version + ".so", libraryName)
          : new NativeLibrary("linux-x86_64", "libyggdrasilffi-" + version + ".so", libraryName);
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

  static final class NativeLibrary {
    private final String platformDirectory;
    private final String fileName;
    private final String libraryName;

    NativeLibrary(String platformDirectory, String fileName, String libraryName) {
      this.platformDirectory = platformDirectory;
      this.fileName = fileName;
      this.libraryName = libraryName;
    }

    String resourcePath() {
      return "/native/" + platformDirectory + "/" + fileName;
    }

    String fileName() {
      return fileName;
    }

    String libraryName() {
      return libraryName;
    }
  }
}
