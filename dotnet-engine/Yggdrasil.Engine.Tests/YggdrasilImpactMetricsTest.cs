using System.Collections.Generic;
using System.Text.Json.Nodes;
using NUnit.Framework;
using Yggdrasil;

public class YggdrasilImpactMetricsTest
{
    [Test]
    public void DefineCounter_Throws_Only_When_Invalid()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        Assert.DoesNotThrow(() => yggdrasilEngine.DefineCounter("server_time", "Measures time spent on server doing things"));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineCounter("server_time", null!));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineCounter(null!, "Measures time spent on server doing things"));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineCounter(null!, null!));
    }

    [Test]
    public void IncCounter_Throws_Only_When_Invalid()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        yggdrasilEngine.DefineCounter("requests_total", "Total requests");

        Assert.DoesNotThrow(() => yggdrasilEngine.IncCounter("requests_total"));
        Assert.DoesNotThrow(() => yggdrasilEngine.IncCounter("requests_total", 5));
        Assert.DoesNotThrow(() => yggdrasilEngine.IncCounter("requests_total", 3, new Dictionary<string, string> { { "env", "test" } }));

        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.IncCounter(null!));
    }

    [Test]
    public void DefineGauge_Throws_Only_When_Invalid()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        Assert.DoesNotThrow(() => yggdrasilEngine.DefineGauge("cpu_usage", "CPU usage"));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineGauge("cpu_usage", null!));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineGauge(null!, "CPU usage"));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineGauge(null!, null!));
    }

    [Test]
    public void SetGauge_Throws_Only_When_Invalid()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        yggdrasilEngine.DefineGauge("queue_depth", "Queue depth");

        Assert.DoesNotThrow(() => yggdrasilEngine.SetGauge("queue_depth", 10.5));
        Assert.DoesNotThrow(() => yggdrasilEngine.SetGauge("queue_depth", 5.25, new Dictionary<string, string> { { "env", "prod" } }));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.SetGauge(null!, 1.0));
    }

    [Test]
    public void DefineHistogram_Throws_Only_When_Invalid()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        Assert.DoesNotThrow(() => yggdrasilEngine.DefineHistogram("request_duration", "Request duration"));
        Assert.DoesNotThrow(() => yggdrasilEngine.DefineHistogram("request_duration_custom", "Request duration custom", new[] { 0.1, 0.5, 1.0, 5.0 }));

        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineHistogram("request_duration", null!));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineHistogram(null!, "Request duration"));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.DefineHistogram(null!, null!));
    }

    [Test]
    public void ObserveHistogram_Throws_Only_When_Invalid()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        yggdrasilEngine.DefineHistogram("request_duration", "Request duration", new[] { 0.1, 0.5, 1.0, 5.0 });

        Assert.DoesNotThrow(() => yggdrasilEngine.ObserveHistogram("request_duration", 0.05));
        Assert.DoesNotThrow(() => yggdrasilEngine.ObserveHistogram("request_duration", 0.75, new Dictionary<string, string> { { "env", "test" } }));
        Assert.Throws<YggdrasilEngineException>(() => yggdrasilEngine.ObserveHistogram(null!, 1.0));
    }

    [Test]
    public void ImpactMetrics_Methods_Can_Be_Used_Together()
    {
        var yggdrasilEngine = new YggdrasilEngine();

        Assert.DoesNotThrow(() =>
        {
            yggdrasilEngine.DefineCounter("test_counter", "Test counter");
            yggdrasilEngine.IncCounter("test_counter", 10);
            yggdrasilEngine.DefineGauge("test_gauge", "Test gauge");
            yggdrasilEngine.SetGauge("test_gauge", 42);
            yggdrasilEngine.DefineHistogram("test_histogram", "Test histogram", new[] { 0.1, 0.5, 1.0 });
            yggdrasilEngine.ObserveHistogram("test_histogram", 0.25);
        });
    }

    [Test]
    public void CollectMetrics_Returns_Recorded_Metric()
    {
        var yggdrasilEngine = new YggdrasilEngine();
        yggdrasilEngine.DefineCounter("requests_total", "Total requests");
        Assert.DoesNotThrow(() => yggdrasilEngine.IncCounter("requests_total"));
        var metrics = yggdrasilEngine.CollectMetricsBucket();
        var counter = JsonNode.Parse(metrics!)!["impact_metrics"]!.AsArray()[0]!;
        var samples = counter["samples"]!.AsArray()![0]!;
        Assert.AreEqual("requests_total", counter["name"]!.GetValue<string>());
        Assert.AreEqual(1, samples["value"]!.GetValue<int>());
    }
}
