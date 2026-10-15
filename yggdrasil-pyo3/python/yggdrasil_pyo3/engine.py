import json
import logging
from dataclasses import dataclass, field
from typing import Any, Callable, Optional

from yggdrasil_pyo3.custom_strategy import CustomStrategyHandler
from yggdrasil_pyo3.yggdrasil_native import NativeEngine, core_version  # type: ignore

_logger = logging.getLogger(__name__)


@dataclass(init=False)
class FeatureToggle:
    """`FeatureToggle` is the result of querying if a feature is enabled."""

    name: str
    """The name of the toggle that was queried."""

    is_enabled: bool = False
    """Whether the feature is enabled for the given context.

    Defaults to `False` when the engine did not know the toggle and no fallback
    resolved a value, or when the evaluation failed.
    """

    is_found: bool = False
    """Whether the engine knew about the toggle at all.

    `False` means the toggle was missing from the engine's state or evaluation
    errored. That is distinct from a known toggle that evaluated to disabled,
    which is `is_found=True, is_enabled=False`.
    """

    requires_impression_event_emission: bool = False
    """Whether the engine expects its caller to emit an impression event.

    These bindings are not concerned with the publishing itself. However,
    the engine is the source of whether a toggle has the publication of
    impression events enabled.

    `False` means that the SDK should not emit impression events. It also means
    the engine could not be asked, either because the lookup itself failed or
    because an earlier step of the evaluation did."""

    def __init__(
        self,
        *,
        name: str,
        is_enabled: bool = False,
        is_found: bool = False,
        requires_impression_event_emission: bool = False,
    ):
        self.name = name
        self.is_enabled = is_enabled
        self.is_found = is_found
        self.requires_impression_event_emission = requires_impression_event_emission

    def __bool__(self):
        raise TypeError(
            f"FeatureToggle for {self.name!r} has no truth value. "
            "Read .is_enabled to check whether the feature is enabled, "
            "or .is_found to check whether the engine knew the toggle."
        )


@dataclass
class Variant:
    name: str
    payload: Optional[dict[str, str]] = field(default_factory=dict)
    enabled: bool = False
    feature_enabled: bool = False

    @staticmethod
    def from_dict(data: dict) -> "Variant":
        return Variant(
            name=data.get("name", ""),
            payload=data.get("payload"),
            enabled=data.get("enabled", False),
            feature_enabled=data.get("featureEnabled", False),
        )


def disabled_variant() -> Variant:
    """The variant the engine falls back to when no variant could be resolved.

    `payload` is `None` rather than an empty dict so that it drops out of
    serialization, matching what the engine itself hands back."""
    return Variant(name="disabled", payload=None, enabled=False, feature_enabled=False)


@dataclass(frozen=True)
class FeatureVariant:
    """`FeatureVariant` is the result of querying which variant a feature resolves to."""

    name: str
    """The name of the toggle that was queried."""

    variant: Variant
    """The variant the toggle resolved to for the given context.

    `get_variant` hands back the disabled variant when the engine did not know
    the toggle, or when the evaluation failed. Note that a known toggle can
    resolve to a variant that looks exactly like it, so read `is_found` to tell
    the two apart.
    """

    is_found: bool
    """Whether the engine knew about the toggle at all.

    `False` means the toggle was missing from the engine's state or evaluation
    errored. That is distinct from a known toggle that resolved to the disabled
    variant, which is `is_found=True`.
    """

    requires_impression_event_emission: bool
    """Whether the engine expects its caller to emit an impression event.

    These bindings are not concerned with the publishing itself. However,
    the engine is the source of whether a toggle has the publication of
    impression events enabled.

    `False` means that the SDK should not emit impression events. It also means
    the engine could not be asked, either because the lookup itself failed or
    because an earlier step of the evaluation did."""


@dataclass
class FeatureDefinition:
    name: str
    project: str
    type: Optional[str]


class PyO3UnleashEngine:
    """The engine on top of the compiled PyO3 module. So far only `take_state`,
    `is_enabled`, `get_variant`, `count_toggle`, `get_metrics`, `get_state`,
    `list_known_toggles` and `get_core_version`.
    """

    @staticmethod
    def get_core_version() -> str:
        """The version of the Yggdrasil core this engine runs, read from the
        compiled module, so it always matches the binary in use."""
        return core_version()

    def __init__(self):
        self._native = NativeEngine()
        self.custom_strategy_handler = CustomStrategyHandler()

    def take_state(self, state_json: str) -> Optional[str]:
        warnings = self._native.take_state(state_json)
        self.custom_strategy_handler.update_strategies(state_json)
        return warnings

    def register_custom_strategies(self, custom_strategies: dict[str, Any]):
        self.custom_strategy_handler.register_custom_strategies(custom_strategies)

    def is_enabled(
        self,
        toggle_name: str,
        context: dict[str, Any],
        *,
        fallback_function: Optional[Callable[[str, dict[str, Any]], Any]] = None,
    ) -> FeatureToggle:
        """Ask whether a feature is enabled, and record the query.

        `fallback_function` is consulted only when the engine does not know the
        toggle; its answer sets `is_enabled` but leaves `is_found` at `False`.

        Never raises: whatever fails, the caller gets the disabled toggle.
        """
        try:
            custom_strategy_results = (
                self.custom_strategy_handler.evaluate_custom_strategies(
                    toggle_name, context
                )
            )
            enabled, impression = self._native.is_enabled(
                toggle_name, context or {}, custom_strategy_results
            )
            is_found = enabled is not None
            if not is_found:
                if fallback_function is not None:
                    enabled = bool(fallback_function(toggle_name, context))
                ## The native engine only records toggles it knows, so record
                ## this one with the answer settled on
                self._native.count_toggle(toggle_name, bool(enabled))
            return FeatureToggle(
                name=toggle_name,
                is_enabled=bool(enabled),
                is_found=is_found,
                requires_impression_event_emission=impression,
            )
        except Exception:
            result = FeatureToggle(name=toggle_name)
            _logger.warning(
                "Failed to evaluate toggle %s, returning %s",
                toggle_name,
                result,
                exc_info=True,
            )
            return result

    def get_variant(self, toggle_name: str, context: dict[str, Any]) -> FeatureVariant:
        """Ask which variant a feature resolves to, and record the query.

        This records both the toggle and the resolved variant against the
        engine's metrics, so they show up in the next `get_metrics()` payload.

        Never raises, and answers all or nothing: whatever fails, the caller
        gets the disabled variant rather than a half built answer.
        """
        try:
            custom_strategy_results = (
                self.custom_strategy_handler.evaluate_custom_strategies(
                    toggle_name, context
                )
            )
            ## One native call evaluates, looks up the impression flag and
            ## records the toggle and the variant (unknown toggles as disabled)
            variant, impression = self._native.get_variant(
                toggle_name, context or {}, custom_strategy_results
            )
            if variant is None:
                return FeatureVariant(
                    name=toggle_name,
                    variant=disabled_variant(),
                    is_found=False,
                    requires_impression_event_emission=impression,
                )
            name, payload, enabled, feature_enabled = variant
            return FeatureVariant(
                name=toggle_name,
                variant=Variant(
                    name=name,
                    payload=payload,
                    enabled=enabled,
                    feature_enabled=feature_enabled,
                ),
                is_found=True,
                requires_impression_event_emission=impression,
            )
        except Exception:
            result = FeatureVariant(
                name=toggle_name,
                variant=disabled_variant(),
                is_found=False,
                requires_impression_event_emission=False,
            )
            _logger.warning(
                "Failed to evaluate variant for toggle %s, returning %s",
                toggle_name,
                result,
                exc_info=True,
            )
            return result

    def count_toggle(self, toggle_name: str, enabled: bool) -> None:
        self._native.count_toggle(toggle_name, enabled)

    def get_metrics(self) -> Optional[dict[str, Any]]:
        metrics = self._native.get_metrics()
        return None if metrics is None else json.loads(metrics)

    def get_state(self) -> str:
        return self._native.get_state()

    def list_known_toggles(self) -> list[FeatureDefinition]:
        return [
            FeatureDefinition(name=name, project=project, type=type_)
            for name, project, type_ in self._native.list_known_toggles()
        ]
