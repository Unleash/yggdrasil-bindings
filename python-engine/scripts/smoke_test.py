"""Smoke test for an installed wheel: the PyO3 module imports and answers.

Also prints the tag of the wheel pip installed, so the log shows which of the
built wheels was picked on this platform.
"""

import platform
import sys
from importlib.metadata import distribution

from yggdrasil_engine.yggdrasil_native import NativeEngine

engine = NativeEngine()
engine.take_state(
    '{"version": 2, "features": [{"name": "f", "enabled": true, "strategies": [{"name": "default"}]}]}'
)
assert engine.check_enabled("f", {}) == (True, False)
assert engine.check_enabled("missing", {}) == (None, False)

wheel_info = distribution("yggdrasil-engine").read_text("WHEEL") or ""
tags = [line.split(": ", 1)[1] for line in wheel_info.splitlines() if line.startswith("Tag:")]
print(
    f"ok: python {sys.version.split()[0]} on {platform.system()} {platform.machine()},"
    f" installed wheel {', '.join(tags)}"
)
