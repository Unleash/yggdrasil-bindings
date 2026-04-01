#if NET8_0_OR_GREATER
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Yggdrasil;

[JsonSerializable(typeof(EngineResponse<object>))]
[JsonSerializable(typeof(System.Text.Json.JsonElement))]
[JsonSerializable(typeof(FeatureCollection))]
[JsonSerializable(typeof(Dictionary<string, bool>))]
[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase)]
internal partial class YggdrasilJsonSerializerContext : JsonSerializerContext
{
}
#endif
