using Yggdrasil;

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
