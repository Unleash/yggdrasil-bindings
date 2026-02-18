package io.getunleash.engine;

import static io.getunleash.engine.TestStrategies.alwaysFails;
import static io.getunleash.engine.TestStrategies.alwaysTrue;
import static io.getunleash.engine.TestStrategies.onlyTrueIfParameterValueMatchesContext;
import static org.assertj.core.api.Assertions.assertThat;
import static org.junit.jupiter.api.Assertions.*;
import static org.junit.jupiter.params.provider.Arguments.of;

import java.io.IOException;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.*;

class CustomStrategiesEvaluatorTest {

  public static Stream<Arguments> invalidNamesAndContext() {
    IStrategy testStrategy = alwaysTrue("test-strategy");
    return Stream.of(
        of(Stream.empty(), null, null),
        of(Stream.empty(), null, new Context()),
        of(Stream.empty(), "", null),
        of(Stream.empty(), "", new Context()),
        of(Stream.of(testStrategy), null, null),
        of(Stream.of(testStrategy), null, new Context()),
        of(Stream.of(testStrategy), "", null),
        of(Stream.of(testStrategy), "", new Context()));
  }

  private static Stream<Arguments> twoStrategies() {
    return Stream.of(
        of(
            alwaysTrue("custom"),
            alwaysTrue("cus-tom"),
            Map.of("customStrategy1", true, "customStrategy2", true)),
        of(
            alwaysFails("custom"),
            alwaysFails("cus-tom"),
            Map.of("customStrategy1", false, "customStrategy2", false)),
        of(
            alwaysTrue("custom"),
            alwaysFails("cus-tom"),
            Map.of("customStrategy1", true, "customStrategy2", false)),
        of(
            alwaysFails("custom"),
            alwaysTrue("cus-tom"),
            Map.of("customStrategy1", false, "customStrategy2", true)),
        of(
            alwaysTrue("wrongName"),
            alwaysTrue("wrongName"),
            Map.of("customStrategy1", false, "customStrategy2", false)),
        of(
            alwaysTrue("custom"),
            alwaysTrue("custom"),
            Map.of("customStrategy1", true, "customStrategy2", false)));
  }

  private static Stream<Arguments> singleStrategy() {
    return Stream.of(
        of("custom", Map.of("customStrategy1", true, "customStrategy2", false)),
        of("cus-tom", Map.of("customStrategy1", false, "customStrategy2", true)),
        of("unknown", Map.of("customStrategy1", false, "customStrategy2", false)));
  }

  @ParameterizedTest
  @MethodSource("invalidNamesAndContext")
  void invalidNameAndContext_shouldEvalToEmpty(
      Stream<IStrategy> strategies, String name, Context context) {
    CustomStrategiesEvaluator customStrategiesEvaluator =
        new CustomStrategiesEvaluator(strategies, new HashSet<>());
    assertThat(customStrategiesEvaluator.eval(name, context)).isEmpty();
  }

  @ParameterizedTest
  @MethodSource("singleStrategy")
  void singleAlwaysTrueStrategy_shouldEvalTo(String strategyName, Map<String, Boolean> expected)
      throws IOException, YggdrasilInvalidInputException {
    UnleashEngine unleashEngine = new UnleashEngine(List.of(alwaysTrue(strategyName)));
    unleashEngine.takeState(ResourceReader.readResourceAsString("custom-strategy-tests.json"));
    assertThat(
            unleashEngine.customStrategiesEvaluatorEval("Feature.Custom.Strategies", new Context()))
        .isEqualTo(expected);
  }

  @ParameterizedTest
  @MethodSource("twoStrategies")
  void twoExistingStrategy_shouldEvalToBothStrategies(
      IStrategy one, IStrategy two, Map<String, Boolean> expected)
      throws IOException, YggdrasilInvalidInputException {
    UnleashEngine unleashEngine = new UnleashEngine(List.of(one, two));
    unleashEngine.takeState(ResourceReader.readResourceAsString("custom-strategy-tests.json"));
    assertThat(
            unleashEngine.customStrategiesEvaluatorEval("Feature.Custom.Strategies", new Context()))
        .isEqualTo(expected);
  }

  @Test
  void faultyStrategy_shouldEvalToFalse() throws IOException, YggdrasilInvalidInputException {
    UnleashEngine unleashEngine = new UnleashEngine(List.of(alwaysFails("custom")));
    unleashEngine.takeState(ResourceReader.readResourceAsString("custom-strategy-tests.json"));
    assertEquals(
        Map.of("customStrategy1", false, "customStrategy2", false),
        unleashEngine.customStrategiesEvaluatorEval("Feature.Custom.Strategies", new Context()));
  }

  @Test
  void repeated_strategy_with_different_parameters_should_evaluate_separately()
      throws IOException, YggdrasilInvalidInputException {
    UnleashEngine unleashEngine =
        new UnleashEngine(
            List.of(onlyTrueIfParameterValueMatchesContext("myFancyStrategy", "myFancy")));
    unleashEngine.takeState(ResourceReader.readResourceAsString("repeated_custom_strategy.json"));
    var context = new Context();
    context.setProperties(Map.of("myFancy", "one"));
    assertThat(unleashEngine.customStrategiesEvaluatorEval("repeated.custom", context)).hasSize(2);
  }
}
