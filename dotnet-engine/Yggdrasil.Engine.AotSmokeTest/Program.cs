using Yggdrasil;

// This program is published with Native AOT (see the .csproj) and executed in CI.
// It exists purely as a regression guard: if anyone reintroduces reflection-based
// JSON serialization or native-library loading into Yggdrasil.Engine, the AOT
// publish will emit trim/AOT analysis warnings (treated as errors) and/or this
// program will crash at runtime. A plain `dotnet build` does not catch either of
// those, because the trim/AOT whole-program analysis only runs on publish.
//
// The happy path below deliberately exercises every AOT-sensitive code path:
//   - `new YggdrasilEngine()`  -> native library extraction + load (NativeLoader)
//   - `TakeState(...)`         -> source-generated JSON deserialization (FeatureCollection)
//   - `GetState()`             -> source-generated JSON serialization (Object)
//   - `IsEnabled(...)`         -> full FFI + flatbuffer roundtrip

const string state =
    "{\"version\":1,\"features\":[{\"name\":\"testFeature\",\"enabled\":true,\"strategies\":[{\"name\":\"default\"}]}]}";

using var engine = new YggdrasilEngine();

engine.TakeState(state);

var retrievedState = engine.GetState();
if (!retrievedState.Contains("testFeature"))
{
    Console.Error.WriteLine($"AOT smoke test FAILED: GetState() did not round-trip the feature. Got: {retrievedState}");
    return 1;
}

var result = engine.IsEnabled("testFeature", new Context());
if (!result.Enabled)
{
    Console.Error.WriteLine("AOT smoke test FAILED: expected 'testFeature' to be enabled.");
    return 1;
}

Console.WriteLine("AOT smoke test PASSED: engine initialised, state round-tripped, and IsEnabled evaluated under Native AOT.");
return 0;
