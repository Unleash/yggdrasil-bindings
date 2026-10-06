"""POC: `is_enabled` through a PyO3 module instead of ctypes + JSON.

`NativeUnleashEngine` mirrors the parts of `UnleashEngine` that `is_enabled`
needs, with the same behaviour, so the two can be compared side by side.
Evaluation, the impression lookup and counting happen in one call into Rust,
and the context dict is read directly, with no JSON.
"""

import logging
from typing import Any, Callable, Optional

from yggdrasil_engine.yggdrasil_native import NativeEngine

from yggdrasil_engine.custom_strategy import CustomStrategyHandler
from yggdrasil_engine.engine import FeatureToggle

_logger = logging.getLogger(__name__)


class NativeUnleashEngine:
    def __init__(self):
        self._native = NativeEngine()
        self.custom_strategy_handler = CustomStrategyHandler()

    def take_state(self, state_json: str) -> Optional[str]:
        warnings = self._native.take_state(state_json)
        self.custom_strategy_handler.update_strategies(state_json)
        return warnings

    def register_custom_strategies(self, custom_strategies: dict):
        self.custom_strategy_handler.register_custom_strategies(custom_strategies)

    def is_enabled(
        self,
        toggle_name: str,
        context: dict,
        *,
        fallback_function: Optional[Callable[[str, dict], Any]] = None,
    ) -> FeatureToggle:
        """Ask whether a feature is enabled, and record the query.

        Same contract as `UnleashEngine.is_enabled`: `fallback_function` is
        consulted only for unknown toggles and leaves `is_found` at `False`,
        the final answer is counted, and it never raises.
        """
        try:
            custom_strategy_results = (
                self.custom_strategy_handler.evaluate_custom_strategies(
                    toggle_name, context
                )
                ## No results means none to pass, which saves converting an empty dict
                or None
            )
            value, impression = self._native.check_enabled(
                toggle_name, context or {}, custom_strategy_results, True
            )
            is_found = value is not None
            if not is_found and fallback_function is not None:
                value = fallback_function(toggle_name, context)
            is_enabled = bool(value)
            ## The engine only counts toggles it knows; unknown ones are counted
            ## here, once the fallback has resolved the value
            if not is_found:
                self.count_toggle(toggle_name, is_enabled)
            return FeatureToggle(
                name=toggle_name,
                is_enabled=is_enabled,
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

    def count_toggle(self, toggle_name: str, enabled: bool):
        self._native.count_toggle(toggle_name, enabled)
