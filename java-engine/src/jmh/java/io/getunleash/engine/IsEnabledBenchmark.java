package io.getunleash.engine;

import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Fork;
import org.openjdk.jmh.annotations.Level;
import org.openjdk.jmh.annotations.Measurement;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.annotations.Threads;
import org.openjdk.jmh.annotations.Warmup;

@BenchmarkMode(Mode.Throughput)
@OutputTimeUnit(TimeUnit.SECONDS)
@Warmup(iterations = 3, time = 1)
@Measurement(iterations = 5, time = 1)
@Fork(2)
public class IsEnabledBenchmark {
  private static final String FEATURE_NAME = "benchmark-feature";

  @State(Scope.Benchmark)
  public static class EngineState {
    UnleashEngine engine;

    @Setup(Level.Trial)
    public void setup() throws YggdrasilInvalidInputException {
      engine = new UnleashEngine();
      engine.takeState(
          "{\"version\":2,\"features\":[{\"name\":\""
              + FEATURE_NAME
              + "\",\"enabled\":true,\"strategies\":[{\"name\":\"flexibleRollout\","
              + "\"parameters\":{\"rollout\":\"100\",\"stickiness\":\"userId\","
              + "\"groupId\":\"benchmark\"}}]}]}");
    }
  }

  @State(Scope.Thread)
  public static class ContextState {
    Context context;

    @Setup(Level.Trial)
    public void setup(EngineState state) throws YggdrasilInvalidInputException {
      context = new Context();
      context.setUserId("benchmark-user");
      context.setSessionId("benchmark-session");
      context.setAppName("java-engine-benchmark");
      context.setEnvironment("development");

      if (!Boolean.TRUE.equals(state.engine.isEnabled(FEATURE_NAME, context).value)) {
        throw new IllegalStateException("Benchmark feature must be enabled");
      }
    }
  }

  @Benchmark
  @Threads(1)
  public FlatResponse<Boolean> isEnabled1Thread(EngineState state, ContextState context)
      throws YggdrasilInvalidInputException {
    return state.engine.isEnabled(FEATURE_NAME, context.context);
  }

  @Benchmark
  @Threads(2)
  public FlatResponse<Boolean> isEnabled2Threads(EngineState state, ContextState context)
      throws YggdrasilInvalidInputException {
    return state.engine.isEnabled(FEATURE_NAME, context.context);
  }

  @Benchmark
  @Threads(4)
  public FlatResponse<Boolean> isEnabled4Threads(EngineState state, ContextState context)
      throws YggdrasilInvalidInputException {
    return state.engine.isEnabled(FEATURE_NAME, context.context);
  }

  @Benchmark
  @Threads(8)
  public FlatResponse<Boolean> isEnabled8Threads(EngineState state, ContextState context)
      throws YggdrasilInvalidInputException {
    return state.engine.isEnabled(FEATURE_NAME, context.context);
  }
}
