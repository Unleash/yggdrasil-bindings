# Java Bindings to Yggdrasil

This project provides a Java wrapper for the Yggdrasil engine, built on top of a native library and hooked together using JNI.

## ☕ Overview

The Java Engine embeds the Yggdrasil engine using Flatbuffers to communicate with the core.

## Usage

The `UnleashEngine` class provides a pure Java wrapper around the Yggdrasil feature evaluation engine. It allows you to evaluate feature toggles and variants entirely in-memory, without calling back to a remote Unleash server at runtime. This is intended to be used as part of a larger project that's capable of fetching the feature toggle definitions.

### Native Library Loading

The Java engine uses JNI and must load a native Yggdrasil library before the engine can be used. By default, the native library is bundled in the JAR and extracted to a temporary file at runtime.

The loader tries native libraries in this order:

1. A path configured with the `io.getunleash.engine.native.path` system property.
2. The bundled native library from the JAR.

The configured path can point either to the exact native library file or to a directory containing the versioned native library for the current platform:

```bash
java -Dio.getunleash.engine.native.path=/opt/unleash/native -jar app.jar
```

For example, on Linux with Yggdrasil core version `0.20.8`, the directory above should contain:

```text
/opt/unleash/native/libyggdrasilffi-0.20.8.so
```

The configured directory must use the versioned native library name for the current platform:

```text
Linux:   libyggdrasilffi-0.20.8.so
macOS:   libyggdrasilffi-0.20.8.dylib
Windows: yggdrasilffi-0.20.8.dll
```

The matching native binaries are also included in the published JAR under `native/<platform>/`. You can inspect them with:

```bash
jar tf yggdrasil-engine-<version>.jar | grep '^native/'
```

You can extract all bundled native binaries with:

```bash
jar xf yggdrasil-engine-<version>.jar native
```

Or extract only the platform you need:

```bash
jar xf yggdrasil-engine-<version>.jar native/linux-x86_64
```

Then point `io.getunleash.engine.native.path` at the extracted platform directory.

### 📥 Loading State

Before evaluating any features, you must initialize the engine with feature toggle definitions. This is done using the `takeState` method. The input to takeState should be the raw JSON response from the Unleash `/api/client/features endpoint`. For example:

``` bash
curl http://unleash-url/api/client/features -H "Authorization: YOUR_API_TOKEN" > toggles.json
```

``` java
Path path = Path.of("toggles.json");

String clientFeaturesJson = Files.readString(path);

UnleashEngine engine = new UnleashEngine();
engine.takeState(clientFeaturesJson);

```

### Querying Toggle State

Once the engine is initialized, you can evaluate toggles using the isEnabled or getVariant methods:

``` java
Context context = new Context();
context.setUserId("user-123");

FlatResponse<Boolean> enabledResponse = engine.isEnabled("some-toggle", context);

if (Boolean.TRUE.equals(enabledResponse.value)) {
    // Feature is enabled for this context
}

if(enabledResponse.impressionData) {
    // Impression data has been enabled in Unleash
}

FlatResponse<VariantDef> response = this.featureRepository.getVariant("some-toggle-with-variants", context);

VariantDef variant = response.value;
if (variant != null) {
    // do something with the variant
}
```

You can also query a list of toggles that's the engine currently knows about:

``` java
List<FeatureDef> toggles = engine.listKnownToggles();
for (FeatureDef toggle : toggles) {
    System.out.println("Toggle: " + toggle.getName());
}
```

## Metrics

Metrics are automatically collected through the isEnabled/getVariant calls. The metrics can be queried back like so:

``` java
MetricsBucket metrics = engine.getMetrics();
```

This will clear the current metrics buffer. This means that if the caller attempts to send this upstream and that call fails, the caller is responsible for retrying.


## Metadata Methods

The engine provides a few methods to retrieve some static metadata about what it supports.

You can query the version of the underlying Yggdrasil engine, which will return a semver string:

``` java
String version = UnleashEngine.getCoreVersion(); //0.18.1
```

You can also retrieve the list of built-in strategies that the engine is aware of:

``` java
List<String> strategies = UnleashEngine.getBuiltInStrategies();
```


## Development

### Prerequisites

To work with this project, you’ll need:

- The Yggdrasil flatbuffer binary (compiled with Rust)
- [flatc version 25.2.10](https://github.com/google/flatbuffers/releases/tag/v25.2.10) for regenerating the Java FlatBuffer bindings

### Linting

```bash
./gradlew spotlessApply
```

### Testing

The FFI library is automatically built on running tests but you'll need to make sure that you've set up the [FFI build correctly](../yggdrasilffi/README.md). We use Gradle here, tests can be invoked with:

``` bash
./gradlew test
```

### FlatBuffer Bindings

The Java engine uses FlatBuffers for communication with the engine core. If you make changes to the data interchange format, regenerate the bindings like this:

``` bash
flatc --java --java-package-prefix "io.getunleash" -o java-engine/src/main/java flat-buffer-defs/enabled-message.fbs

```

You'll need to update the [FFI](../yggdrasilffi/) code as well to handle any changes you make to the FlatBuffer definitions.

### Building a JAR for Testing

You may want to build a JAR for linking to a local project for testing. To do this, make sure you've followed the [instructions for building](../yggdrasilffi/README.md) the FFI code.

You can then run the local publish

``` bash
./gradlew publishToMavenLocal
```

This will build a JAR and make it available in your local maven repository
