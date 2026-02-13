using System;
using System.Text.Json;
using NUnit.Framework;
using Yggdrasil;
using Yggdrasil.Test;

public class YggdrasilImpactMetricsTest
{
    private JsonSerializerOptions options = new JsonSerializerOptions
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    [Test]
    public void DefineCounter_Throws_Only_When_Invalid()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        Assert.DoesNotThrow(() => yggdrasilEngine.DefineCounter("server_time", "Measures time spent on server doing things"));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineCounter("server_time", null!));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineCounter(null!, "Measures time spent on server doing things"));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineCounter(null!, null!));
    }
}
