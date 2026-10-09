import json
import logging
import os
from unittest.mock import Mock

import pytest

from yggdrasil_pyo3.engine import FeatureToggle, PyO3UnleashEngine

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


## copied from python-engine/tests/test_engine.py (`UnleashEngine` → `PyO3UnleashEngine`;
## the variant and metrics parts left out, as this package has neither yet)


def test_client_spec():
    unleash_engine = PyO3UnleashEngine()

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


def test_custom_strategies_work_end_to_end():
    engine = PyO3UnleashEngine()

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

    assert enabled_when_better.is_enabled is True
    assert disabled_when_not_better.is_enabled is False


def test_is_enabled_ignores_callback_value_if_engine_responded():
    engine = PyO3UnleashEngine()

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
    engine = PyO3UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: True
    )

    assert result.is_enabled is True


def test_is_enabled_ignores_callback_value_when_engine_returns_false():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("disabledFeature", False))

    result = engine.is_enabled(
        "disabledFeature", {}, fallback_function=lambda name, ctx: True
    )

    assert result.is_enabled is False


def test_is_enabled_fallback_returning_false_is_respected():
    engine = PyO3UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: False
    )

    assert result.is_enabled is False


def test_is_enabled_fallback_receives_toggle_name_and_context():
    engine = PyO3UnleashEngine()
    context = {"userId": "123"}
    fallback = Mock(return_value=True)

    engine.is_enabled("nonExistentFeature", context, fallback_function=fallback)

    fallback.assert_called_once_with("nonExistentFeature", context)


def test_is_enabled_fallback_not_invoked_when_toggle_found():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))
    fallback = Mock(return_value=True)

    engine.is_enabled("testFeature", {}, fallback_function=fallback)

    fallback.assert_not_called()


def test_is_enabled_returns_feature_toggle_for_enabled_toggle():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    result = engine.is_enabled("testFeature", {})

    assert result == FeatureToggle(name="testFeature", is_enabled=True, is_found=True)


def test_is_enabled_returns_found_feature_toggle_when_toggle_is_disabled():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("disabledFeature", False))

    result = engine.is_enabled("disabledFeature", {})

    assert result == FeatureToggle(
        name="disabledFeature", is_enabled=False, is_found=True
    )


def test_is_enabled_reports_not_found_when_fallback_resolves_value():
    engine = PyO3UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: True
    )

    assert result.name == "nonExistentFeature"
    assert result.is_enabled is True
    assert result.is_found is False


def test_is_enabled_names_the_queried_toggle():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    assert engine.is_enabled("testFeature", {}).name == "testFeature"


def test_is_enabled_coerces_fallback_value_to_bool():
    engine = PyO3UnleashEngine()

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=lambda name, ctx: "truthy"
    )

    assert result.is_enabled is True


def test_feature_toggle_cannot_be_used_as_a_bool():
    with pytest.raises(TypeError):
        bool(FeatureToggle(name="testFeature", is_enabled=True, is_found=True))


def test_is_enabled_result_cannot_be_used_as_a_bool():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    result = engine.is_enabled("testFeature", {})

    with pytest.raises(TypeError):
        if result:
            pass


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
    engine = PyO3UnleashEngine()
    engine.take_state(_impression_state("testFeature", True, True))

    result = engine.is_enabled("testFeature", {})

    assert result.requires_impression_event_emission is True


def test_is_enabled_does_not_report_impression_event_without_impression_data():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    result = engine.is_enabled("testFeature", {})

    assert result.requires_impression_event_emission is False


def test_is_enabled_reports_impression_event_for_disabled_toggle():
    engine = PyO3UnleashEngine()
    engine.take_state(_impression_state("disabledFeature", False, True))

    result = engine.is_enabled("disabledFeature", {})

    ## Impression events are emitted for disabled evaluations too, so the
    ## lookup must not be gated on the evaluation result
    assert result.is_enabled is False
    assert result.requires_impression_event_emission is True


def test_is_enabled_defaults_to_disabled_when_fallback_raises():
    engine = PyO3UnleashEngine()

    def exploding_fallback(name, ctx):
        raise RuntimeError("bad fallback")

    result = engine.is_enabled(
        "nonExistentFeature", {}, fallback_function=exploding_fallback
    )

    assert result.is_enabled is False
    assert result.is_found is False
    ## The original also checks that nothing was counted, through get_metrics,
    ## which this package doesn't have yet


def test_is_enabled_fallback_function_must_be_passed_as_keyword():
    engine = PyO3UnleashEngine()
    engine.take_state(_toggle_state("testFeature", True))

    ## This TypeError comes from binding the arguments, before any of the
    ## never-raises handling inside is_enabled can run
    try:
        engine.is_enabled("testFeature", {}, lambda name, ctx: True)
        assert False, "expected TypeError for positional fallback_function"
    except TypeError:
        pass

## not copied: python-engine's never-raise tests force errors by patching the
## ctypes engine's internals, which don't exist here, and it doesn't test a
## missing context

def test_is_enabled_does_not_raise_when_the_context_cannot_be_read(
    caplog: pytest.LogCaptureFixture,
):
    engine = PyO3UnleashEngine()
    _ = engine.take_state(_toggle_state("testFeature", True))

    with caplog.at_level(logging.WARNING, logger="yggdrasil_pyo3.engine"):
        result = engine.is_enabled("testFeature", {"plan": object()})

    assert result == FeatureToggle(name="testFeature")
    assert "Failed to evaluate toggle testFeature" in caplog.text


def test_is_enabled_treats_a_missing_context_as_empty_and_gives_it_to_the_fallback_as_is():
    engine = PyO3UnleashEngine()
    _ = engine.take_state(_toggle_state("testFeature", True))
    fallback = Mock(return_value=True)

    known = engine.is_enabled("testFeature", None)
    _ = engine.is_enabled("nonExistentFeature", None, fallback_function=fallback)

    assert known.is_enabled is True
    fallback.assert_called_once_with("nonExistentFeature", None)
