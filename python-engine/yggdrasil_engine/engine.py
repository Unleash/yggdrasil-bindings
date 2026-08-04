import ctypes
import json
import logging
import os
import platform
from contextlib import contextmanager
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, ClassVar, Dict, List, Optional, Type, TypeVar, cast

from yggdrasil_engine.custom_strategy import CustomStrategyHandler


def _get_binary_path():
    lib_dir = os.path.join(os.path.dirname(__file__), "lib")
    system = platform.system()

    if system == "Linux":
        return os.path.join(lib_dir, "libyggdrasilffi.so")
    elif system == "Darwin":
        return os.path.join(lib_dir, "libyggdrasilffi.dylib")
    elif system == "Windows":
        return os.path.join(lib_dir, "yggdrasilffi.dll")
    else:
        raise RuntimeError(f"Unsupported operating system: {system}")


T = TypeVar("T")

_logger = logging.getLogger(__name__)


class StatusCode(Enum):
    OK = "Ok"
    NOT_FOUND = "NotFound"
    ERROR = "Error"


class YggdrasilError(Exception):
    pass


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
    payload: Optional[Dict[str, str]] = field(default_factory=dict)
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
    return Variant(name="disabled", payload=None, enabled=False, feature_enabled=False)


"""The variant the engine falls back to when no variant could be resolved.

`payload` is `None` rather than an empty dict so that it drops out of
serialization, matching what the engine itself hands back."""


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

    @staticmethod
    def from_dict(data: dict) -> "FeatureDefinition":
        return FeatureDefinition(
            name=data.get("name", ""),
            project=data.get("project", ""),
            type=data.get("type"),
        )


def load_feature_defs(raw_defs: List[dict]) -> List[FeatureDefinition]:
    return [FeatureDefinition.from_dict(defn) for defn in raw_defs]


@dataclass
class Response:
    status_code: StatusCode
    value: Optional[any]
    error_message: Optional[str]

    deserializers: ClassVar[Dict[Type, Callable[[Any], Any]]] = {
        Variant: Variant.from_dict,
        List[FeatureDefinition]: load_feature_defs,
    }

    @staticmethod
    def from_json(data: str, value_type: Type[T]) -> "Response[T]":
        status_code = StatusCode(data["status_code"])
        raw_value = data.get("value")
        error_message = data.get("error_message")

        if raw_value is not None:
            if value_type in Response.deserializers:
                value = Response.deserializers[value_type](raw_value)
            else:
                value = cast(value_type, raw_value)
        else:
            value = None

        return Response(
            status_code=status_code, value=value, error_message=error_message
        )


class UnleashEngine:
    def __init__(self):
        binary_path = _get_binary_path()

        self.lib = ctypes.CDLL(binary_path)
        self.lib.new_engine.restype = ctypes.c_void_p
        self.lib.take_state.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        self.lib.take_state.restype = ctypes.POINTER(ctypes.c_char)
        self.lib.check_enabled.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        self.lib.check_enabled.restype = ctypes.POINTER(ctypes.c_char)
        self.lib.check_variant.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        self.lib.check_variant.restype = ctypes.POINTER(ctypes.c_char)
        self.lib.free_engine.argtypes = [ctypes.c_void_p]
        self.lib.free_engine.restype = None
        self.lib.free_response.argtypes = [ctypes.c_void_p]
        self.lib.free_response.restype = None

        self.lib.get_metrics.argtypes = [ctypes.c_void_p]
        self.lib.get_metrics.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.get_state.argtypes = [ctypes.c_void_p]
        self.lib.get_state.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.count_toggle.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_bool,
        ]
        self.lib.count_toggle.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.count_variant.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        self.lib.count_variant.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.should_emit_impression_event.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
        ]

        self.lib.should_emit_impression_event.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.list_known_toggles.argtypes = [ctypes.c_void_p]
        self.lib.list_known_toggles.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.define_counter.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        self.lib.define_counter.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.inc_counter.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_int64,
            ctypes.c_char_p,
        ]
        self.lib.inc_counter.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.collect_impact_metrics.argtypes = [ctypes.c_void_p]
        self.lib.collect_impact_metrics.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.restore_impact_metrics.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
        ]
        self.lib.restore_impact_metrics.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.define_gauge.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        self.lib.define_gauge.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.set_gauge.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_double,
            ctypes.c_char_p,
        ]
        self.lib.set_gauge.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.define_histogram.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        self.lib.define_histogram.restype = ctypes.POINTER(ctypes.c_char)

        self.lib.observe_histogram.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_double,
            ctypes.c_char_p,
        ]
        self.lib.observe_histogram.restype = ctypes.POINTER(ctypes.c_char)

        self.state = self.lib.new_engine()
        self.custom_strategy_handler = CustomStrategyHandler()

    def __del__(self):
        if hasattr(self, "state") and self.state is not None:
            self.lib.free_engine(self.state)

    @contextmanager
    def materialize_pointer(self, ptr, value_type: Type[T]):
        try:
            response = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
            yield Response.from_json(json.loads(response), value_type)
        finally:
            self.lib.free_response(ptr)

    def take_state(self, state_json: str) -> Optional[List[Warning]]:
        response_ptr = self.lib.take_state(self.state, state_json.encode("utf-8"))
        self.custom_strategy_handler.update_strategies(state_json)
        with self.materialize_pointer(response_ptr, List[Warning]) as result:
            if result.value:
                warnings = "\n".join(
                    [f"{warning.toggle_name}: {warning.message}" for warning in result]
                )
                return warnings
            return None

    def get_state(self) -> str:
        response_ptr = self.lib.get_state(self.state)
        with self.materialize_pointer(response_ptr, dict) as result:
            return json.dumps(result.value)

    def is_enabled(
        self,
        toggle_name: str,
        context: dict,
        *,
        fallback_function: Optional[Callable[[str, dict], Any]] = None,
    ) -> FeatureToggle:
        result = FeatureToggle(name=toggle_name)
        try:
            value = self._do_is_enabled(toggle_name, context)
            is_found = value is not None

            if not is_found and fallback_function is not None:
                value = fallback_function(toggle_name, context)

            enabled = bool(value)
            result = FeatureToggle(
                name=toggle_name,
                is_enabled=enabled,
                is_found=is_found,
            )

            self.count_toggle(toggle_name, enabled)
            result.requires_impression_event_emission = bool(
                self.should_emit_impression_event(toggle_name)
            )
        except Exception:
            _logger.warning(
                "Failed to fully evaluate toggle %s, returning %s",
                toggle_name,
                result,
                exc_info=True,
            )
        return result

    def get_variant(self, toggle_name: str, context: dict) -> FeatureVariant:
        try:
            value = self._do_get_variant(toggle_name, context)
            variant = value if value is not None else disabled_variant()

            result = FeatureVariant(
                name=toggle_name,
                variant=variant,
                is_found=value is not None,
                requires_impression_event_emission=bool(
                    self.should_emit_impression_event(toggle_name)
                ),
            )

            self.count_toggle(toggle_name, variant.feature_enabled)
            self.count_variant(toggle_name, variant.name)

            return result
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

    def register_custom_strategies(self, custom_strategies: dict):
        self.custom_strategy_handler.register_custom_strategies(custom_strategies)

    def count_toggle(self, toggle_name: str, enabled: bool):
        response_ptr = self.lib.count_toggle(
            self.state, toggle_name.encode("utf-8"), enabled
        )
        self.lib.free_response(response_ptr)

    def count_variant(self, toggle_name: str, variant_name: str):
        response_ptr = self.lib.count_variant(
            self.state, toggle_name.encode("utf-8"), variant_name.encode("utf-8")
        )
        self.lib.free_response(response_ptr)

    def get_metrics(self) -> Dict[str, Any]:
        metrics_ptr = self.lib.get_metrics(self.state)
        with self.materialize_pointer(metrics_ptr, Dict[str, Any]) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)
            return response.value

    def should_emit_impression_event(self, toggle_name: str) -> bool:
        response_ptr = self.lib.should_emit_impression_event(
            self.state, toggle_name.encode("utf-8")
        )
        with self.materialize_pointer(response_ptr, bool) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)
            return response.value

    def list_known_toggles(self) -> List[FeatureDefinition]:
        response_ptr = self.lib.list_known_toggles(self.state)
        with self.materialize_pointer(
            response_ptr, List[FeatureDefinition]
        ) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)
            return response.value

    def define_counter(self, name: str, help_text: str) -> None:
        response_ptr = self.lib.define_counter(
            self.state,
            name.encode("utf-8"),
            help_text.encode("utf-8"),
        )
        with self.materialize_pointer(response_ptr, type(None)) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)

    def inc_counter(
        self, name: str, value: int = 1, labels: Optional[Dict[str, str]] = None
    ) -> None:
        labels_json = json.dumps(labels).encode("utf-8") if labels else None
        response_ptr = self.lib.inc_counter(
            self.state,
            name.encode("utf-8"),
            value,
            labels_json,
        )
        with self.materialize_pointer(response_ptr, type(None)) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)

    def collect_impact_metrics(self) -> List[Dict[str, Any]]:
        response_ptr = self.lib.collect_impact_metrics(self.state)
        with self.materialize_pointer(response_ptr, List[Dict[str, Any]]) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)
            return response.value or []

    def restore_impact_metrics(self, metrics: List[Dict[str, Any]]) -> None:
        metrics_json = json.dumps(metrics).encode("utf-8")
        response_ptr = self.lib.restore_impact_metrics(
            self.state,
            metrics_json,
        )
        with self.materialize_pointer(response_ptr, type(None)) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)

    def define_gauge(self, name: str, help_text: str) -> None:
        response_ptr = self.lib.define_gauge(
            self.state,
            name.encode("utf-8"),
            help_text.encode("utf-8"),
        )
        with self.materialize_pointer(response_ptr, type(None)) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)

    def set_gauge(
        self, name: str, value: float, labels: Optional[Dict[str, str]] = None
    ) -> None:
        labels_json = json.dumps(labels).encode("utf-8") if labels else None
        response_ptr = self.lib.set_gauge(
            self.state,
            name.encode("utf-8"),
            value,
            labels_json,
        )
        with self.materialize_pointer(response_ptr, type(None)) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)

    def define_histogram(
        self, name: str, help_text: str, buckets: Optional[List[float]] = None
    ) -> None:
        buckets_json = json.dumps(buckets if buckets is not None else []).encode(
            "utf-8"
        )
        response_ptr = self.lib.define_histogram(
            self.state,
            name.encode("utf-8"),
            help_text.encode("utf-8"),
            buckets_json,
        )
        with self.materialize_pointer(response_ptr, type(None)) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)

    def observe_histogram(
        self, name: str, value: float, labels: Optional[Dict[str, str]] = None
    ) -> None:
        labels_json = json.dumps(labels).encode("utf-8") if labels else None
        response_ptr = self.lib.observe_histogram(
            self.state,
            name.encode("utf-8"),
            value,
            labels_json,
        )
        with self.materialize_pointer(response_ptr, type(None)) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)

    def _do_is_enabled(self, toggle_name: str, context: dict) -> Optional[bool]:
        serialized_context = json.dumps(context or {})
        custom_strategy_results = json.dumps(
            self.custom_strategy_handler.evaluate_custom_strategies(
                toggle_name, context
            )
        )

        response_ptr = self.lib.check_enabled(
            self.state,
            toggle_name.encode("utf-8"),
            serialized_context.encode("utf-8"),
            custom_strategy_results.encode("utf-8"),
        )
        with self.materialize_pointer(response_ptr, bool) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)
            ## `None` is the engine saying it does not know this toggle
            return response.value

    def _do_get_variant(self, toggle_name: str, context: dict) -> Optional[Variant]:
        serialized_context = json.dumps(context or {})
        custom_strategy_results = json.dumps(
            self.custom_strategy_handler.evaluate_custom_strategies(
                toggle_name, context
            )
        )

        response_ptr = self.lib.check_variant(
            self.state,
            toggle_name.encode("utf-8"),
            serialized_context.encode("utf-8"),
            custom_strategy_results.encode("utf-8"),
        )
        with self.materialize_pointer(response_ptr, Variant) as response:
            if response.status_code == StatusCode.ERROR:
                raise YggdrasilError(response.error_message)
            ## `None` is the engine saying it does not know this toggle
            return response.value
