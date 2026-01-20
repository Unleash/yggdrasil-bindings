from dataclasses import asdict
import json
from yggdrasil_engine.engine import UnleashEngine, Variant, FeatureDefinition
import json
import os


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
        print(unleash_engine.get_variant(toggle_name, context))


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

            result = unleash_engine.is_enabled(toggle_name, context) or False

            assert (
                result == expected_result
            ), f"Failed test '{test['description']}': expected {expected_result}, got {result}"

        for test in suite_data.get("variantTests", []):
            context = test["context"]
            toggle_name = test["toggleName"]
            expected_result = test["expectedResult"]

            result = unleash_engine.get_variant(toggle_name, context) or Variant(
                "disabled", None, False, False
            )

            ## We get away with this right now because the casing in the spec tests for feature_enabled
            ## is snake_case. At some point this is going to change to camel case and this is going to break
            expected_json = json.dumps(expected_result)
            actual_json = json.dumps(variant_to_dict(result))

            assert (
                expected_json == actual_json
            ), f"Failed test '{test['description']}': expected {expected_json}, got {actual_json}"


def test_custom_strategies_work_end_to_end():
    engine = UnleashEngine()

    class BreadStrategy:
        def apply(self, _parameters, context):
            return context.get("betterThanSlicedBread") == True

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

    assert enabled_when_better == True
    assert disabled_when_not_better == False
    assert should_be_sour_dough.name == "sourDough"


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
    assert 'status_code' not in empty_state
    assert 'error_message' not in empty_state
    
    test_state = {
        "version": 1,
        "features": [{"name": "testFeature", "enabled": True, "strategies": [{"name": "default"}]}]
    }

    engine.take_state(json.dumps(test_state))
    retrieved_state = engine.get_state()

    assert "testFeature" in retrieved_state
    assert '"version": 1' in retrieved_state
    assert '"name": "testFeature"' in retrieved_state
    assert '"name": "default"' in retrieved_state
    assert 'status_code' not in retrieved_state
    assert 'error_message' not in retrieved_state


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
    test_sample = next((s for s in counter["samples"] if s["labels"].get("env") == "test"), None)
    prod_sample = next((s for s in counter["samples"] if s["labels"].get("env") == "prod"), None)
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
    test_sample = next((s for s in gauge["samples"] if s["labels"].get("env") == "test"), None)
    prod_sample = next((s for s in gauge["samples"] if s["labels"].get("env") == "prod"), None)
    assert test_sample["value"] == 5
    assert prod_sample["value"] == 3


def test_observe_histogram_observes_values():
    engine = UnleashEngine()
    engine.define_histogram("request_duration", "Request duration", [0.1, 0.5, 1.0, 5.0])
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
    engine.define_histogram("request_duration", "Request duration", [0.1, 0.5, 1.0, 5.0])
    engine.observe_histogram("request_duration", 0.05, {"env": "test"})
    engine.observe_histogram("request_duration", 0.75, {"env": "prod"})

    metrics = engine.collect_impact_metrics()
    histogram = next((m for m in metrics if m["name"] == "request_duration"), None)

    assert histogram is not None
    assert len(histogram["samples"]) == 2
    test_sample = next((s for s in histogram["samples"] if s["labels"].get("env") == "test"), None)
    prod_sample = next((s for s in histogram["samples"] if s["labels"].get("env") == "prod"), None)
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
    restored_counter = next((m for m in restored_metrics if m["name"] == "test_counter"), None)
    restored_gauge = next((m for m in restored_metrics if m["name"] == "test_gauge"), None)
    restored_histogram = next((m for m in restored_metrics if m["name"] == "test_histogram"), None)
    assert restored_counter["samples"][0]["value"] == 10
    assert restored_gauge["samples"][0]["value"] == 42
    assert restored_histogram is not None
