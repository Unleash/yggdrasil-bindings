import json
import os
from dataclasses import FrozenInstanceError, asdict
from unittest.mock import Mock

import pytest

from yggdrasil_engine.engine import (
    FeatureDefinition,
    FeatureToggle,
    FeatureVariant,
    UnleashEngine,
    Variant,
    YggdrasilError,
    disabled_variant,
)

CUSTOM_STRATEGY_STATE = """
{
    "version": 1,
    "features": [
        {
            "name": "Feature.A",
            "enabled": true,
            "strategies": [
                {
                    "name": "breadStrategy",
                    "parameters": {}
                }
            ],
            "variants": [
                {
                    "name": "sourDough",
                    "weight": 100
                }
            ],
            "impressionData": true
        }
    ]
}
"""


def variant_to_dict(variant) -> dict:
    print(variant)
    return {k: v for k, v in asdict(variant).items() if v is not None}


def test_get_variant_does_not_crash():
    unleash_engine = UnleashEngine()

    with open("../test-data/simple.json") as file:
        state = json.load(file)
        context = {"userId": "123"}
        toggle_name = "testToggle"

        unleash_engine.take_state(json.dumps(state))
        result = unleash_engine.get_variant(toggle_name, context)

        ## simple.json does not carry testToggle, so this exercises the
        ## substituted disabled variant against a real state file
        assert result == FeatureVariant(
            name="testToggle",
            variant=disabled_variant(),
            is_found=False,
            requires_impression_event_emission=False,
        )


def test_client_spec():
    unleash_engine = UnleashEngine()

    with open("../client-specification/specifications/index.json", "r") as file:
        test_suites = json.load(file)

    for suite in test_suites:
        suite_path = os.path.join("../client-specification/specifications", suite)

        with open(suite_path, "r") as suite_file:
            suite_data = json.load(suite_file)

        unleash_engine.take_state(json.dumps(suite_data["state"]))

        for test in suite_data.get("tests", []):
            context = test["context"]
            toggle_name = test["toggleName"]
            expected_result = test["expectedResult"]

            result = unleash_engine.is_enabled(toggle_name, context).is_enabled or False

            assert result == expected_result, (
                f"Failed test '{test['description']}': expected {expected_result}, got {result}"
            )

        for test in suite_data.get("variantTests", []):
            context = test["context"]
            toggle_name = test["toggleName"]
            expected_result = test["expectedResult"]

            result = unleash_engine.get_variant(toggle_name, context)

            ## We get away with this right now because the casing in the spec tests for feature_enabled
            ## is snake_case. At some point this is going to change to camel case and this is going to break
            expected_json = json.dumps(expected_result)
            actual_json = json.dumps(variant_to_dict(result.variant))

            assert expected_json == actual_json, (
                f"Failed test '{test['description']}': expected {expected_json}, got {actual_json}"
            )


def test_custom_strategies_work_end_to_end():
    engine = UnleashEngine()

    class BreadStrategy:
        def apply(self, _parameters, context):
            return context.get("betterThanSlicedBread") is True

    engine.register_custom_strategies({"breadStrategy": BreadStrategy()})
    engine.take_state(CUSTOM_STRATEGY_STATE)

    enabled_when_better = engine.is_enabled(
        "Feature.A", {"betterThanSlicedBread": True}
    )
    disabled_when_not_better = engine.is_enabled(
        "Feature.A", {"betterThanSlicedBread": False}
    )

    should_be_sour_dough = engine.get_variant(
        "Feature.A", {"betterThanSlicedBread": True}
    )

    assert enabled_when_better.is_enabled is True
    assert disabled_when_not_better.is_enabled is False
    assert should_be_sour_dough.variant.name == "sourDough"
    assert should_be_sour_dough.is_found is True


def test_increments_counts_for_yes_no_and_variants():
    engine = UnleashEngine()

    with open("../test-data/simple.json") as file:
        state = json.load(file)

    engine.take_state(json.dumps(state))

    engine.count_toggle("testToggle", True)
    engine.count_toggle("testToggle", True)
    engine.count_toggle("testToggle", False)
    engine.count_variant("testToggle", "disabled")

    metrics = engine.get_metrics()

    assert metrics["toggles"]["testToggle"]["yes"] == 2
    assert metrics["toggles"]["testToggle"]["no"] == 1
    assert metrics["toggles"]["testToggle"]["variants"]["disabled"] == 1


def test_metrics_are_flushed_when_get_metrics_is_called():
    engine = UnleashEngine()

    with open("../test-data/simple.json") as file:
        state = json.load(file)

    engine.take_state(json.dumps(state))

    engine.count_toggle("testToggle", True)

    metrics = engine.get_metrics()
    assert metrics["toggles"]["testToggle"]["yes"] == 1

    metrics = engine.get_metrics()
    assert metrics is None


def test_metrics_are_still_incremented_when_toggle_does_not_exist():
    engine = UnleashEngine()

    engine.count_toggle("aToggleSoSecretItDoesNotExist", True)

    metrics = engine.get_metrics()

    assert metrics["toggles"]["aToggleSoSecretItDoesNotExist"]["yes"] == 1


def test_yields_impression_data():
    engine = UnleashEngine()

    engine.take_state(CUSTOM_STRATEGY_STATE)

    assert engine.should_emit_impression_event("Feature.A")
    assert not engine.should_emit_impression_event("Nonexisting")


def test_list_known_toggles():
    engine = UnleashEngine()

    engine.take_state(CUSTOM_STRATEGY_STATE)
    first_toggle = engine.list_known_toggles()[0]

    assert len(engine.list_known_toggles()) == 1
    assert first_toggle == FeatureDefinition(
        name="Feature.A", project="default", type=None
    )


def test_list_empty_toggles_yields_empty_list():
    engine = UnleashEngine()

    assert engine.list_known_toggles() == []


def test_get_state_and_roundtrip():
    """Test get_state returns valid JSON and supports roundtrip"""
    engine = UnleashEngine()

    empty_state = engine.get_state()
    assert '"features": []' in empty_state
    assert "status_code" not in empty_state
    assert "error_message" not in empty_state

    test_state = {
        "version": 1,
        "features": [
            {
                "name": "testFeature",
                "enabled": True,
                "strategies": [{"name": "default"}],
            }
        ],
    }

    engine.take_state(json.dumps(test_state))
    retrieved_state = engine.get_state()

    assert "testFeature" in retrieved_state
    assert '"version": 1' in retrieved_state
    assert '"name": "testFeature"' in retrieved_state
    assert '"name": "default"' in retrieved_state
    assert "status_code" not in retrieved_state
    assert "error_message" not in retrieved_state


def test_is_enabled_counts_toggle():
    engine = UnleashEngine()

    state = {
        "version": 1,
        "features": [
            {
                "name": "testFeature",
                "enabled": True,
                "strategies": [{"name": "default"}],
            }
        ],
    }
    engine.take_state(json.dumps(state))

    engine.is_enabled("testFeature", {})

    metrics = engine.get_metrics()

    assert metrics is not None
    assert metrics["toggles"]["testFeature"]["yes"] == 1


def test_is_enabled_ignores_callback_value_if_engine_responded():
    engine = UnleashEngine()

    state = {
        "version": 1,
        "features": [
            {
                "name": "testFeature",
                "enabled": True,
                "strategies": [{"name": "default"}],
            }
        ],
    }
    engine.take_state(json.dumps(state))

    result = engine.is_enabled(
        "testFeature", {}, fallback_function=lambda name, ctx: False
    )

    assert result.is_enabled is True
    assert result.is_found is True


def test_is_enabled_returns_callback_value_if_engine_did_not_respond():
    engine = UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: True
    )

    assert result.is_enabled is True


def _toggle_state(name: str, enabled: bool) -> str:
    return json.dumps(
        {
            "version": 1,
            "features": [
                {"name": name, "enabled": enabled, "strategies": [{"name": "default"}]}
            ],
        }
    )


def _impression_state(name: str, enabled: bool, impression_data: bool) -> str:
    return json.dumps(
        {
            "version": 1,
            "features": [
                {
                    "name": name,
                    "enabled": enabled,
                    "strategies": [{"name": "default"}],
                    "impressionData": impression_data,
                }
            ],
        }
    )


def _variant_state(name: str, enabled: bool, variant_name: str, payload=None) -> str:
    return json.dumps(
        {
            "version": 1,
            "features": [
                {
                    "name": name,
                    "enabled": enabled,
                    "strategies": [{"name": "default"}],
                    ## A single variant at full weight makes resolution deterministic
                    "variants": [
                        {"name": variant_name, "weight": 100, "payload": payload}
                    ],
                }
            ],
        }
    )


def _variant_impression_state(
    name: str, enabled: bool, variant_name: str, impression_data: bool
) -> str:
    return json.dumps(
        {
            "version": 1,
            "features": [
                {
                    "name": name,
                    "enabled": enabled,
                    "strategies": [{"name": "default"}],
                    "variants": [{"name": variant_name, "weight": 100}],
                    "impressionData": impression_data,
                }
            ],
        }
    )


def test_is_enabled_ignores_callback_value_when_engine_returns_false():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("disabledFeature", False))

    result = engine.is_enabled(
        "disabledFeature", {}, fallback_function=lambda name, ctx: True
    )

    assert result.is_enabled is False


def test_is_enabled_fallback_returning_false_is_respected():
    engine = UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: False
    )

    assert result.is_enabled is False


def test_is_enabled_fallback_receives_toggle_name_and_context():
    engine = UnleashEngine()
    context = {"userId": "123"}
    fallback = Mock(return_value=True)

    engine.is_enabled("nonExistentFeature", context, fallback_function=fallback)

    fallback.assert_called_once_with("nonExistentFeature", context)


def test_is_enabled_fallback_not_invoked_when_toggle_found():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))
    fallback = Mock(return_value=True)

    engine.is_enabled("testFeature", {}, fallback_function=fallback)

    fallback.assert_not_called()


def test_is_enabled_counts_fallback_resolved_value_not_raw_response():
    engine = UnleashEngine()

    engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: True
    )

    metrics = engine.get_metrics()

    assert metrics["toggles"]["nonExistentFeature"]["yes"] == 1
    assert metrics["toggles"]["nonExistentFeature"].get("no", 0) == 0


def test_is_enabled_reports_not_found_when_toggle_missing_without_fallback():
    engine = UnleashEngine()

    result = engine.is_enabled("nonExistentFeature", {})

    metrics = engine.get_metrics()

    assert result == FeatureToggle(
        name="nonExistentFeature", is_enabled=False, is_found=False
    )
    assert metrics["toggles"]["nonExistentFeature"]["no"] == 1


def test_is_enabled_counts_disabled_toggle_as_no():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("disabledFeature", False))

    engine.is_enabled("disabledFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["disabledFeature"]["no"] == 1
    assert metrics["toggles"]["disabledFeature"].get("yes", 0) == 0


def test_is_enabled_returns_feature_toggle_for_enabled_toggle():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    result = engine.is_enabled("testFeature", {})

    assert result == FeatureToggle(name="testFeature", is_enabled=True, is_found=True)


def test_is_enabled_returns_found_feature_toggle_when_toggle_is_disabled():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("disabledFeature", False))

    result = engine.is_enabled("disabledFeature", {})

    assert result == FeatureToggle(
        name="disabledFeature", is_enabled=False, is_found=True
    )


def test_is_enabled_reports_not_found_when_fallback_resolves_value():
    engine = UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: True
    )

    assert result.name == "nonExistentFeature"
    assert result.is_enabled is True
    assert result.is_found is False


def test_is_enabled_names_the_queried_toggle():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    assert engine.is_enabled("testFeature", {}).name == "testFeature"


def test_is_enabled_coerces_fallback_value_to_bool():
    engine = UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: "truthy"
    )

    assert result.is_enabled is True


def test_feature_toggle_cannot_be_used_as_a_bool():
    with pytest.raises(TypeError):
        bool(FeatureToggle(name="testFeature", is_enabled=True, is_found=True))


def test_is_enabled_result_cannot_be_used_as_a_bool():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    result = engine.is_enabled("testFeature", {})

    with pytest.raises(TypeError):
        if result:
            pass


def test_is_enabled_accumulates_counts_across_multiple_calls():
    engine = UnleashEngine()
    state = {
        "version": 1,
        "features": [
            {
                "name": "enabledFeature",
                "enabled": True,
                "strategies": [{"name": "default"}],
            },
            {
                "name": "disabledFeature",
                "enabled": False,
                "strategies": [{"name": "default"}],
            },
        ],
    }
    engine.take_state(json.dumps(state))

    engine.is_enabled("enabledFeature", {})
    engine.is_enabled("enabledFeature", {})
    engine.is_enabled("disabledFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["enabledFeature"]["yes"] == 2
    assert metrics["toggles"]["enabledFeature"].get("no", 0) == 0
    assert metrics["toggles"]["disabledFeature"]["no"] == 1
    assert metrics["toggles"]["disabledFeature"].get("yes", 0) == 0


def test_is_enabled_returns_disabled_toggle_when_engine_errors(monkeypatch):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_is_enabled",
        Mock(side_effect=YggdrasilError("boom")),
    )

    result = engine.is_enabled("testFeature", {})

    assert result == FeatureToggle(name="testFeature", is_enabled=False, is_found=False)


def test_is_enabled_does_not_count_the_toggle_when_engine_errors(monkeypatch):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_is_enabled",
        Mock(side_effect=YggdrasilError("boom")),
    )

    engine.is_enabled("testFeature", {})

    ## Nothing after the raise is attempted, so there is nothing to count
    assert engine.get_metrics() is None


def test_is_enabled_does_not_raise_on_unexpected_engine_failure(monkeypatch):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_is_enabled",
        Mock(side_effect=RuntimeError("kaboom")),
    )

    result = engine.is_enabled("testFeature", {})

    assert result.is_enabled is False
    assert result.is_found is False


def test_is_enabled_does_not_use_the_fallback_when_engine_errors(monkeypatch):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_is_enabled",
        Mock(side_effect=YggdrasilError("boom")),
    )
    fallback = Mock(return_value=True)

    result = engine.is_enabled("testFeature", {}, fallback_function=fallback)

    fallback.assert_not_called()
    assert result.is_enabled is False
    assert result.is_found is False


def test_is_enabled_defaults_to_disabled_when_fallback_raises():
    engine = UnleashEngine()

    def exploding_fallback(name, ctx):
        raise RuntimeError("bad fallback")

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=exploding_fallback
    )

    assert result.is_enabled is False
    assert result.is_found is False
    ## A raising fallback leaves no usable value, so there is nothing to count
    assert engine.get_metrics() is None


def test_is_enabled_fallback_function_must_be_passed_as_keyword():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    ## This TypeError comes from binding the arguments, before any of the
    ## never-raises handling inside is_enabled can run
    try:
        engine.is_enabled("testFeature", {}, lambda name, ctx: True)
        assert False, "expected TypeError for positional fallback_function"
    except TypeError:
        pass


def test_inc_counter_increments_value():
    engine = UnleashEngine()
    engine.define_counter("test_counter", "Test counter")
    engine.inc_counter("test_counter", 5)
    engine.inc_counter("test_counter", 3)

    metrics = engine.collect_impact_metrics()
    counter = next((m for m in metrics if m["name"] == "test_counter"), None)

    assert counter is not None
    assert counter["help"] == "Test counter"
    assert len(counter["samples"]) == 1
    assert counter["samples"][0]["value"] == 8


def test_inc_counter_with_labels():
    engine = UnleashEngine()
    engine.define_counter("test_counter", "Test counter")
    engine.inc_counter("test_counter", 5, {"env": "test"})
    engine.inc_counter("test_counter", 3, {"env": "prod"})

    metrics = engine.collect_impact_metrics()
    counter = next((m for m in metrics if m["name"] == "test_counter"), None)

    assert counter is not None
    assert len(counter["samples"]) == 2
    test_sample = next(
        (s for s in counter["samples"] if s["labels"].get("env") == "test"), None
    )
    prod_sample = next(
        (s for s in counter["samples"] if s["labels"].get("env") == "prod"), None
    )
    assert test_sample["value"] == 5
    assert prod_sample["value"] == 3


def test_set_gauge_sets_value():
    engine = UnleashEngine()
    engine.define_gauge("test_gauge", "Test gauge")
    engine.set_gauge("test_gauge", 5)
    engine.set_gauge("test_gauge", 10)

    metrics = engine.collect_impact_metrics()
    gauge = next((m for m in metrics if m["name"] == "test_gauge"), None)

    assert gauge is not None
    assert gauge["help"] == "Test gauge"
    assert len(gauge["samples"]) == 1
    assert gauge["samples"][0]["value"] == 10


def test_set_gauge_with_labels():
    engine = UnleashEngine()
    engine.define_gauge("test_gauge", "Test gauge")
    engine.set_gauge("test_gauge", 5, {"env": "test"})
    engine.set_gauge("test_gauge", 3, {"env": "prod"})

    metrics = engine.collect_impact_metrics()
    gauge = next((m for m in metrics if m["name"] == "test_gauge"), None)

    assert gauge is not None
    assert len(gauge["samples"]) == 2
    test_sample = next(
        (s for s in gauge["samples"] if s["labels"].get("env") == "test"), None
    )
    prod_sample = next(
        (s for s in gauge["samples"] if s["labels"].get("env") == "prod"), None
    )
    assert test_sample["value"] == 5
    assert prod_sample["value"] == 3


def test_observe_histogram_observes_values():
    engine = UnleashEngine()
    engine.define_histogram(
        "request_duration", "Request duration", [0.1, 0.5, 1.0, 5.0]
    )
    engine.observe_histogram("request_duration", 0.05)
    engine.observe_histogram("request_duration", 0.75)
    engine.observe_histogram("request_duration", 3.0)

    metrics = engine.collect_impact_metrics()
    histogram = next((m for m in metrics if m["name"] == "request_duration"), None)

    assert histogram is not None
    assert histogram["help"] == "Request duration"
    assert histogram["type"] == "histogram"
    assert len(histogram["samples"]) == 1


def test_observe_histogram_with_labels():
    engine = UnleashEngine()
    engine.define_histogram(
        "request_duration", "Request duration", [0.1, 0.5, 1.0, 5.0]
    )
    engine.observe_histogram("request_duration", 0.05, {"env": "test"})
    engine.observe_histogram("request_duration", 0.75, {"env": "prod"})

    metrics = engine.collect_impact_metrics()
    histogram = next((m for m in metrics if m["name"] == "request_duration"), None)

    assert histogram is not None
    assert len(histogram["samples"]) == 2
    test_sample = next(
        (s for s in histogram["samples"] if s["labels"].get("env") == "test"), None
    )
    prod_sample = next(
        (s for s in histogram["samples"] if s["labels"].get("env") == "prod"), None
    )
    assert test_sample is not None
    assert prod_sample is not None


def test_define_histogram_with_default_buckets():
    engine = UnleashEngine()
    engine.define_histogram("request_duration", "Request duration")
    engine.observe_histogram("request_duration", 0.05)

    metrics = engine.collect_impact_metrics()
    histogram = next((m for m in metrics if m["name"] == "request_duration"), None)

    assert histogram is not None
    assert histogram["type"] == "histogram"
    assert len(histogram["samples"]) == 1


def test_collect_impact_metrics_returns_empty_list_when_no_metrics():
    engine = UnleashEngine()
    metrics = engine.collect_impact_metrics()
    assert metrics == []


def test_restore_impact_metrics():
    engine = UnleashEngine()
    engine.define_counter("test_counter", "Test counter")
    engine.inc_counter("test_counter", 10)
    engine.define_gauge("test_gauge", "Test gauge")
    engine.set_gauge("test_gauge", 42)
    engine.define_histogram("test_histogram", "Test histogram", [0.1, 0.5, 1.0])
    engine.observe_histogram("test_histogram", 0.25)

    metrics = engine.collect_impact_metrics()
    assert len(metrics) == 3
    counter = next((m for m in metrics if m["name"] == "test_counter"), None)
    gauge = next((m for m in metrics if m["name"] == "test_gauge"), None)
    histogram = next((m for m in metrics if m["name"] == "test_histogram"), None)
    assert counter["samples"][0]["value"] == 10
    assert gauge["samples"][0]["value"] == 42
    assert histogram is not None

    engine.restore_impact_metrics(metrics)

    restored_metrics = engine.collect_impact_metrics()
    assert len(restored_metrics) == 3
    restored_counter = next(
        (m for m in restored_metrics if m["name"] == "test_counter"), None
    )
    restored_gauge = next(
        (m for m in restored_metrics if m["name"] == "test_gauge"), None
    )
    restored_histogram = next(
        (m for m in restored_metrics if m["name"] == "test_histogram"), None
    )
    assert restored_counter["samples"][0]["value"] == 10
    assert restored_gauge["samples"][0]["value"] == 42
    assert restored_histogram is not None


def test_feature_toggle_defaults_to_no_impression_event():
    assert FeatureToggle(name="testFeature").requires_impression_event_emission is False


def test_feature_toggle_rejects_positional_arguments():
    with pytest.raises(TypeError):
        FeatureToggle("testFeature")


def test_feature_toggle_equality_includes_impression_event_flag():
    calls_for_emission = FeatureToggle(
        name="testFeature",
        is_enabled=True,
        is_found=True,
        requires_impression_event_emission=True,
    )
    does_not_call_for_emission = FeatureToggle(
        name="testFeature",
        is_enabled=True,
        is_found=True,
        requires_impression_event_emission=False,
    )

    assert calls_for_emission != does_not_call_for_emission


def test_is_enabled_reports_impression_event_when_toggle_has_impression_data():
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))

    result = engine.is_enabled("testFeature", {})

    assert result.requires_impression_event_emission is True


def test_is_enabled_does_not_report_impression_event_without_impression_data():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    result = engine.is_enabled("testFeature", {})

    assert result.requires_impression_event_emission is False


def test_is_enabled_reports_impression_event_for_disabled_toggle():
    engine = UnleashEngine()
    engine.take_state(_impression_state("disabledFeature", False, True))

    result = engine.is_enabled("disabledFeature", {})

    ## Impression events are emitted for disabled evaluations too, so the
    ## lookup must not be gated on the evaluation result
    assert result.is_enabled is False
    assert result.requires_impression_event_emission is True


def test_is_enabled_does_not_report_impression_event_for_unknown_toggle():
    engine = UnleashEngine()

    result = engine.is_enabled("nonExistentFeature", {})

    assert result.is_found is False
    assert result.requires_impression_event_emission is False


def test_is_enabled_returns_the_engines_impression_event_answer(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))
    monkeypatch.setattr(engine, "should_emit_impression_event", Mock(return_value=True))

    result = engine.is_enabled("testFeature", {})

    ## The state carries no impressionData, so a True here can only come from the engine
    assert result.requires_impression_event_emission is True


def test_is_enabled_returns_the_engines_impression_event_denial(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    monkeypatch.setattr(
        engine, "should_emit_impression_event", Mock(return_value=False)
    )

    result = engine.is_enabled("testFeature", {})

    assert result.requires_impression_event_emission is False


def test_is_enabled_asks_the_engine_for_the_queried_toggle(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    should_emit = Mock(return_value=True)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    engine.is_enabled("testFeature", {})

    should_emit.assert_called_once_with("testFeature")


def test_is_enabled_coerces_impression_event_flag_to_bool(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))
    monkeypatch.setattr(engine, "should_emit_impression_event", Mock(return_value=None))

    result = engine.is_enabled("testFeature", {})

    assert result.requires_impression_event_emission is False


def test_is_enabled_asks_the_engine_even_when_toggle_is_unknown(monkeypatch):
    engine = UnleashEngine()
    should_emit = Mock(return_value=False)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    engine.is_enabled("nonExistentFeature", {})

    should_emit.assert_called_once_with("nonExistentFeature")


def test_is_enabled_asks_the_engine_when_a_fallback_resolves_the_value(monkeypatch):
    engine = UnleashEngine()
    should_emit = Mock(return_value=False)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: True
    )

    should_emit.assert_called_once_with("nonExistentFeature")
    assert result.is_enabled is True
    assert result.is_found is False
    assert result.requires_impression_event_emission is False


def test_is_enabled_does_not_ask_for_impression_data_when_evaluation_errors(
    monkeypatch,
):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_is_enabled",
        Mock(side_effect=YggdrasilError("boom")),
    )
    should_emit = Mock(return_value=True)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    result = engine.is_enabled("testFeature", {})

    ## Without an evaluation there is nothing to emit an impression event about
    should_emit.assert_not_called()
    assert result.requires_impression_event_emission is False


def test_is_enabled_defaults_impression_event_to_false_when_lookup_errors(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    monkeypatch.setattr(
        engine,
        "should_emit_impression_event",
        Mock(side_effect=YggdrasilError("boom")),
    )

    result = engine.is_enabled("testFeature", {})

    ## A failing impression lookup must not disturb the evaluation itself
    assert result.requires_impression_event_emission is False
    assert result.is_enabled is True
    assert result.is_found is True


def test_is_enabled_defaults_impression_event_to_false_on_unexpected_lookup_failure(
    monkeypatch,
):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    monkeypatch.setattr(
        engine,
        "should_emit_impression_event",
        Mock(side_effect=RuntimeError("kaboom")),
    )

    result = engine.is_enabled("testFeature", {})

    assert result.requires_impression_event_emission is False
    assert result.is_enabled is True
    assert result.is_found is True


def test_is_enabled_still_counts_the_toggle_when_impression_lookup_fails(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    monkeypatch.setattr(
        engine,
        "should_emit_impression_event",
        Mock(side_effect=YggdrasilError("boom")),
    )

    result = engine.is_enabled("testFeature", {})

    metrics = engine.get_metrics()

    assert result.requires_impression_event_emission is False
    assert metrics["toggles"]["testFeature"]["yes"] == 1


def test_is_enabled_returns_the_evaluation_when_counting_fails(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    monkeypatch.setattr(
        engine, "count_toggle", Mock(side_effect=YggdrasilError("boom"))
    )

    result = engine.is_enabled("testFeature", {})

    ## A metrics failure must not discard an evaluation that already succeeded
    assert result.is_enabled is True
    assert result.is_found is True


def test_is_enabled_does_not_raise_on_unexpected_counting_failure(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    monkeypatch.setattr(
        engine, "count_toggle", Mock(side_effect=RuntimeError("kaboom"))
    )

    result = engine.is_enabled("testFeature", {})

    assert result.is_enabled is True
    assert result.is_found is True


def test_is_enabled_does_not_ask_for_impression_data_when_counting_fails(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))
    monkeypatch.setattr(
        engine, "count_toggle", Mock(side_effect=YggdrasilError("boom"))
    )
    should_emit = Mock(return_value=True)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    result = engine.is_enabled("testFeature", {})

    should_emit.assert_not_called()
    assert result.requires_impression_event_emission is False


def test_disabled_variant_is_the_engines_disabled_variant():
    ## The payload is None rather than {} so that it drops out of serialization
    assert disabled_variant() == Variant(
        name="disabled", payload=None, enabled=False, feature_enabled=False
    )


def test_feature_variant_cannot_be_mutated():
    result = FeatureVariant(
        name="testFeature",
        variant=disabled_variant(),
        is_found=False,
        requires_impression_event_emission=False,
    )

    with pytest.raises(FrozenInstanceError):
        result.is_found = True


def test_feature_variant_equality_includes_the_resolved_variant():
    sour_dough = FeatureVariant(
        name="Feature.A",
        variant=Variant("sourDough", None, True, True),
        is_found=True,
        requires_impression_event_emission=False,
    )
    rye = FeatureVariant(
        name="Feature.A",
        variant=Variant("rye", None, True, True),
        is_found=True,
        requires_impression_event_emission=False,
    )

    assert sour_dough != rye


def test_feature_variant_equality_includes_impression_event_flag():
    calls_for_emission = FeatureVariant(
        name="Feature.A",
        variant=Variant("sourDough", None, True, True),
        is_found=True,
        requires_impression_event_emission=True,
    )
    does_not_call_for_emission = FeatureVariant(
        name="Feature.A",
        variant=Variant("sourDough", None, True, True),
        is_found=True,
        requires_impression_event_emission=False,
    )

    assert calls_for_emission != does_not_call_for_emission


def test_get_variant_does_not_share_one_disabled_variant_instance():
    engine = UnleashEngine()

    first = engine.get_variant("firstFeature", {})
    second = engine.get_variant("secondFeature", {})

    first.variant.name = "mutated"

    ## Variant is mutable, so handing every substitution the same object would
    ## let one caller poison every later result; disabled_variant() builds a
    ## fresh one per call for exactly that reason
    assert second.variant.name == "disabled"
    assert disabled_variant().name == "disabled"


def test_get_variant_returns_the_resolved_variant_for_an_enabled_toggle():
    engine = UnleashEngine()
    engine.take_state(
        _variant_state(
            "testFeature", True, "sourDough", {"type": "string", "value": "crusty"}
        )
    )

    result = engine.get_variant("testFeature", {})

    assert result == FeatureVariant(
        name="testFeature",
        variant=Variant(
            name="sourDough",
            payload={"type": "string", "value": "crusty"},
            enabled=True,
            feature_enabled=True,
        ),
        is_found=True,
        requires_impression_event_emission=False,
    )


def test_get_variant_names_the_queried_toggle():
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))

    assert engine.get_variant("testFeature", {}).name == "testFeature"


def test_get_variant_reports_found_for_an_enabled_toggle_without_variants():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    result = engine.get_variant("testFeature", {})

    assert result.is_found is True
    assert result.variant == Variant(
        name="disabled", payload=None, enabled=False, feature_enabled=True
    )


def test_get_variant_reports_found_for_a_known_but_disabled_toggle():
    engine = UnleashEngine()
    engine.take_state(_variant_state("disabledFeature", False, "sourDough"))

    result = engine.get_variant("disabledFeature", {})

    ## A disabled toggle resolves to a variant that looks exactly like the
    ## substituted one, so is_found must come from the engine's status code
    assert result.variant == disabled_variant()
    assert result.is_found is True


def test_get_variant_resolves_the_variant_for_the_given_context():
    engine = UnleashEngine()
    state = {
        "version": 1,
        "features": [
            {
                "name": "testFeature",
                "enabled": True,
                "strategies": [{"name": "default"}],
                "variants": [
                    {"name": "sourDough", "weight": 100},
                    {
                        "name": "rye",
                        "weight": 0,
                        "overrides": [{"contextName": "userId", "values": ["123"]}],
                    },
                ],
            }
        ],
    }
    engine.take_state(json.dumps(state))

    result = engine.get_variant("testFeature", {"userId": "123"})

    ## Only the override can select the zero-weight variant, so this can only
    ## hold if the context reached the engine
    assert result.variant.name == "rye"


def test_get_variant_does_not_accept_a_fallback_function():
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))

    ## Unlike is_enabled, there is no user supplied fallback; the disabled
    ## variant is the only substitution
    with pytest.raises(TypeError):
        engine.get_variant("testFeature", {}, fallback_function=lambda name, ctx: None)


def test_get_variant_returns_the_disabled_variant_for_an_unknown_toggle():
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))

    result = engine.get_variant("nonExistentFeature", {})

    assert result == FeatureVariant(
        name="nonExistentFeature",
        variant=disabled_variant(),
        is_found=False,
        requires_impression_event_emission=False,
    )


def test_get_variant_returns_the_disabled_variant_when_the_engine_has_no_state():
    engine = UnleashEngine()

    result = engine.get_variant("nonExistentFeature", {})

    assert result.variant == disabled_variant()
    assert result.is_found is False


def test_get_variant_counts_the_resolved_variant():
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))

    engine.get_variant("testFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["testFeature"]["variants"]["sourDough"] == 1


def test_get_variant_counts_an_enabled_toggle_as_yes():
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))

    engine.get_variant("testFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["testFeature"]["yes"] == 1
    assert metrics["toggles"]["testFeature"].get("no", 0) == 0


def test_get_variant_counts_a_disabled_toggle_as_no():
    engine = UnleashEngine()
    engine.take_state(_variant_state("disabledFeature", False, "sourDough"))

    engine.get_variant("disabledFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["disabledFeature"]["no"] == 1
    assert metrics["toggles"]["disabledFeature"].get("yes", 0) == 0


def test_get_variant_counts_an_enabled_toggle_without_variants_as_yes():
    engine = UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    engine.get_variant("testFeature", {})

    metrics = engine.get_metrics()

    ## The variant is disabled but the feature is not, so the toggle count
    ## follows feature_enabled rather than the variant's own enabled flag
    assert metrics["toggles"]["testFeature"]["variants"]["disabled"] == 1
    assert metrics["toggles"]["testFeature"]["yes"] == 1


def test_get_variant_counts_the_substituted_disabled_variant_for_an_unknown_toggle():
    engine = UnleashEngine()

    engine.get_variant("nonExistentFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["nonExistentFeature"]["variants"]["disabled"] == 1


def test_get_variant_counts_an_unknown_toggle_as_no():
    engine = UnleashEngine()

    engine.get_variant("nonExistentFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["nonExistentFeature"]["no"] == 1
    assert metrics["toggles"]["nonExistentFeature"].get("yes", 0) == 0


def test_get_variant_accumulates_counts_across_multiple_calls():
    engine = UnleashEngine()
    state = {
        "version": 1,
        "features": [
            {
                "name": "enabledFeature",
                "enabled": True,
                "strategies": [{"name": "default"}],
                "variants": [{"name": "sourDough", "weight": 100}],
            },
            {
                "name": "disabledFeature",
                "enabled": False,
                "strategies": [{"name": "default"}],
                "variants": [{"name": "rye", "weight": 100}],
            },
        ],
    }
    engine.take_state(json.dumps(state))

    engine.get_variant("enabledFeature", {})
    engine.get_variant("enabledFeature", {})
    engine.get_variant("disabledFeature", {})

    metrics = engine.get_metrics()

    assert metrics["toggles"]["enabledFeature"]["variants"]["sourDough"] == 2
    assert metrics["toggles"]["enabledFeature"]["yes"] == 2
    assert metrics["toggles"]["disabledFeature"]["variants"]["disabled"] == 1
    assert metrics["toggles"]["disabledFeature"]["no"] == 1


def test_get_variant_reports_impression_event_when_toggle_has_impression_data():
    engine = UnleashEngine()
    engine.take_state(_variant_impression_state("testFeature", True, "sourDough", True))

    result = engine.get_variant("testFeature", {})

    assert result.requires_impression_event_emission is True


def test_get_variant_does_not_report_impression_event_without_impression_data():
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))

    result = engine.get_variant("testFeature", {})

    assert result.requires_impression_event_emission is False


def test_get_variant_reports_impression_event_for_a_disabled_toggle():
    engine = UnleashEngine()
    engine.take_state(
        _variant_impression_state("disabledFeature", False, "sourDough", True)
    )

    result = engine.get_variant("disabledFeature", {})

    ## Impression events are emitted for disabled evaluations too, so the
    ## lookup must not be gated on the resolved variant
    assert result.variant == disabled_variant()
    assert result.requires_impression_event_emission is True


def test_get_variant_does_not_report_impression_event_for_unknown_toggle():
    engine = UnleashEngine()

    result = engine.get_variant("nonExistentFeature", {})

    assert result.is_found is False
    assert result.requires_impression_event_emission is False


def test_get_variant_asks_the_engine_for_the_queried_toggle(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_impression_state("testFeature", True, "sourDough", True))
    should_emit = Mock(return_value=True)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    engine.get_variant("testFeature", {})

    should_emit.assert_called_once_with("testFeature")


def test_get_variant_asks_the_engine_even_when_toggle_is_unknown(monkeypatch):
    engine = UnleashEngine()
    should_emit = Mock(return_value=False)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    engine.get_variant("nonExistentFeature", {})

    should_emit.assert_called_once_with("nonExistentFeature")


def test_get_variant_returns_the_engines_impression_event_answer(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(engine, "should_emit_impression_event", Mock(return_value=True))

    result = engine.get_variant("testFeature", {})

    ## The state carries no impressionData, so a True here can only come from the engine
    assert result.requires_impression_event_emission is True


def test_get_variant_returns_the_engines_impression_event_denial(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_impression_state("testFeature", True, "sourDough", True))
    monkeypatch.setattr(
        engine, "should_emit_impression_event", Mock(return_value=False)
    )

    result = engine.get_variant("testFeature", {})

    assert result.requires_impression_event_emission is False


def test_get_variant_coerces_impression_event_flag_to_bool(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(engine, "should_emit_impression_event", Mock(return_value=None))

    result = engine.get_variant("testFeature", {})

    assert result.requires_impression_event_emission is False


def test_get_variant_returns_the_disabled_variant_when_engine_errors(monkeypatch):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_get_variant",
        Mock(side_effect=YggdrasilError("boom")),
    )

    result = engine.get_variant("testFeature", {})

    assert result == FeatureVariant(
        name="testFeature",
        variant=disabled_variant(),
        is_found=False,
        requires_impression_event_emission=False,
    )


def test_get_variant_does_not_count_when_engine_errors(monkeypatch):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_get_variant",
        Mock(side_effect=YggdrasilError("boom")),
    )

    engine.get_variant("testFeature", {})

    ## Nothing after the raise is attempted, so there is nothing to count
    assert engine.get_metrics() is None


def test_get_variant_does_not_raise_on_unexpected_engine_failure(monkeypatch):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_get_variant",
        Mock(side_effect=RuntimeError("kaboom")),
    )

    result = engine.get_variant("testFeature", {})

    assert result.variant == disabled_variant()
    assert result.is_found is False


def test_get_variant_does_not_ask_for_impression_data_when_evaluation_errors(
    monkeypatch,
):
    engine = UnleashEngine()
    monkeypatch.setattr(
        engine,
        "_do_get_variant",
        Mock(side_effect=YggdrasilError("boom")),
    )
    should_emit = Mock(return_value=True)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    result = engine.get_variant("testFeature", {})

    ## Without an evaluation there is nothing to emit an impression event about
    should_emit.assert_not_called()
    assert result.requires_impression_event_emission is False


def test_get_variant_discards_the_evaluation_when_the_impression_lookup_errors(
    monkeypatch,
):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(
        engine,
        "should_emit_impression_event",
        Mock(side_effect=YggdrasilError("boom")),
    )

    result = engine.get_variant("testFeature", {})

    ## The impression lookup is part of building the result, so its failure
    ## takes the whole evaluation down to the fallback
    assert result == FeatureVariant(
        name="testFeature",
        variant=disabled_variant(),
        is_found=False,
        requires_impression_event_emission=False,
    )


def test_get_variant_discards_the_evaluation_on_unexpected_lookup_failure(
    monkeypatch,
):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(
        engine,
        "should_emit_impression_event",
        Mock(side_effect=RuntimeError("kaboom")),
    )

    result = engine.get_variant("testFeature", {})

    assert result == FeatureVariant(
        name="testFeature",
        variant=disabled_variant(),
        is_found=False,
        requires_impression_event_emission=False,
    )


def test_get_variant_does_not_count_when_the_impression_lookup_fails(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(
        engine,
        "should_emit_impression_event",
        Mock(side_effect=YggdrasilError("boom")),
    )

    result = engine.get_variant("testFeature", {})

    ## The impression lookup runs before either count, so nothing is recorded
    assert result.requires_impression_event_emission is False
    assert engine.get_metrics() is None


def test_get_variant_discards_the_evaluation_when_counting_fails(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(
        engine, "count_variant", Mock(side_effect=YggdrasilError("boom"))
    )

    result = engine.get_variant("testFeature", {})

    ## A metrics failure discards the evaluation that preceded it
    assert result == FeatureVariant(
        name="testFeature",
        variant=disabled_variant(),
        is_found=False,
        requires_impression_event_emission=False,
    )


def test_get_variant_does_not_raise_on_unexpected_counting_failure(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(
        engine, "count_toggle", Mock(side_effect=RuntimeError("kaboom"))
    )

    result = engine.get_variant("testFeature", {})

    assert result == FeatureVariant(
        name="testFeature",
        variant=disabled_variant(),
        is_found=False,
        requires_impression_event_emission=False,
    )


def test_get_variant_asks_for_impression_data_before_counting(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_impression_state("testFeature", True, "sourDough", True))
    monkeypatch.setattr(
        engine, "count_toggle", Mock(side_effect=YggdrasilError("boom"))
    )
    should_emit = Mock(return_value=True)
    monkeypatch.setattr(engine, "should_emit_impression_event", should_emit)

    result = engine.get_variant("testFeature", {})

    ## The lookup lands before counting, but the counting failure still drops
    ## the result it fed, so the answer never reaches the caller
    should_emit.assert_called_once_with("testFeature")
    assert result.requires_impression_event_emission is False


def test_get_variant_counts_the_toggle_before_the_variant(monkeypatch):
    engine = UnleashEngine()
    engine.take_state(_variant_state("testFeature", True, "sourDough"))
    monkeypatch.setattr(
        engine, "count_variant", Mock(side_effect=YggdrasilError("boom"))
    )

    engine.get_variant("testFeature", {})

    ## The toggle is counted first, so it lands even though the variant did not
    metrics = engine.get_metrics()
    assert metrics["toggles"]["testFeature"]["yes"] == 1
    assert metrics["toggles"]["testFeature"]["variants"] == {}
