"""Smoke test for an installed wheel"""

import json
import logging
import platform
import re
import sys
from importlib.metadata import distribution

from yggdrasil_engine.engine import FeatureToggle, UnleashEngine


class PlanStrategy:
    def apply(self, parameters, context):
        return context.get("plan") == parameters.get("plan")


assert re.fullmatch(r"\d+\.\d+\.\d+", UnleashEngine.get_core_version())

engine = UnleashEngine()
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
                {
                    "name": "with_variant",
                    "enabled": True,
                    "strategies": [{"name": "default"}],
                    "variants": [{"name": "blue", "weight": 1000}],
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

assert engine.get_variant("with_variant", {}).variant.name == "blue"
assert engine.get_metrics()["toggles"]["on"]["yes"] == 1

warnings = []
handler = logging.Handler()
handler.emit = warnings.append
engine_logger = logging.getLogger("yggdrasil_engine")
engine_logger.addHandler(handler)
engine_logger.propagate = False
assert engine.is_enabled("on", {"plan": object()}) == FeatureToggle(name="on")
assert len(warnings) == 1 and warnings[0].levelno == logging.WARNING

wheel_info = distribution("yggdrasil-engine").read_text("WHEEL") or ""
tags = [
    line.split(": ", 1)[1]
    for line in wheel_info.splitlines()
    if line.startswith("Tag:")
]
print(
    f"ok: python {sys.version.split()[0]} on {platform.system()} {platform.machine()},"
    f" installed wheel {', '.join(tags)}"
)
