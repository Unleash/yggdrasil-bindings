import logging
from dataclasses import dataclass
from typing import Any, Callable, Optional

# to be removed into Rust
from yggdrasil_pyo3.custom_strategy import CustomStrategyHandler
from yggdrasil_pyo3.yggdrasil_native import NativeEngine  # type: ignore

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


class PyO3UnleashEngine:
    """The engine on top of the compiled PyO3 module. So far only `take_state`
    and `is_enabled`.
    """

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
