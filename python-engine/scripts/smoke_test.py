"""Smoke test for an installed wheel: the PyO3 engine works through
`NativeUnleashEngine` (native_engine.py), the way the SDK would call it.

Also prints the tag of the wheel pip installed, so the log shows which of the
built wheels was picked on this platform.
"""

import json
import logging
import platform
import sys
from importlib.metadata import distribution

from yggdrasil_engine.engine import FeatureToggle
from yggdrasil_engine.native_engine import NativeUnleashEngine


class PlanStrategy:
    def apply(self, parameters, context):
        return context.get("plan") == parameters.get("plan")


engine = NativeUnleashEngine()
engine.register_custom_strategies({"plan": PlanStrategy()})
engine.take_state(
    json.dumps(
        {
            "version": 2,
            "features": [
                {"name": "on", "enabled": True, "strategies": [{"name": "default"}]},
                {
                    "name": "with_impressions",
                    "enabled": True,
                    "impressionData": True,
                    "strategies": [{"name": "default"}],
                },
                {
                    "name": "custom",
                    "enabled": True,
                    "strategies": [{"name": "plan", "parameters": {"plan": "pro"}}],
                },
            ],
        }
    )
)

assert engine.is_enabled("on", {"userId": "1"}) == FeatureToggle(
    name="on", is_enabled=True, is_found=True
)
assert engine.is_enabled("with_impressions", {}).requires_impression_event_emission

assert engine.is_enabled("custom", {"plan": "pro"}).is_enabled
assert not engine.is_enabled("custom", {"plan": "free"}).is_enabled

assert engine.is_enabled(
    "missing", {}, fallback_function=lambda _name, _context: True
) == FeatureToggle(name="missing", is_enabled=True, is_found=False)

warnings = []
handler = logging.Handler()
handler.emit = warnings.append
engine_logger = logging.getLogger("yggdrasil_engine")
engine_logger.addHandler(handler)
engine_logger.propagate = False
assert engine.is_enabled("on", {"plan": object()}) == FeatureToggle(name="on")
assert len(warnings) == 1 and warnings[0].levelno == logging.WARNING

wheel_info = distribution("yggdrasil-engine").read_text("WHEEL") or ""
tags = [line.split(": ", 1)[1] for line in wheel_info.splitlines() if line.startswith("Tag:")]
print(
    f"ok: python {sys.version.split()[0]} on {platform.system()} {platform.machine()},"
    f" installed wheel {', '.join(tags)}"
)
