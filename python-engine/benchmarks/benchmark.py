"""Benchmark `is_enabled` across three engines, side by side:

- current:  today's engine (ctypes + JSON), yggdrasil_engine.engine.UnleashEngine
- reporter: the issue reporter's PyO3 POC, ct_unleash_engine.UnleashEngine,
            timed as eval + impression + count (three native
            calls), like its own bench.py "full" row
- pyo3_poc: our PyO3 POC, yggdrasil_engine.native_engine.NativeUnleashEngine
- pyo3_pythonize: the same POC, but Rust reads the context with pythonize
            (through Context's own deserializer) instead of the hand-written
            reader

3 flags (simple -> complex) x 3 contexts (minimal -> complex), single
thread, plus 8 threads on the rollout flag and standard context.

Env: CALLS (default 50,000), THREADS (8),
PER_THREAD (20,000).
"""

import json
import os
import sys
import threading
import time

from ct_unleash_engine import UnleashEngine as ReporterEngine
from yggdrasil_engine.engine import UnleashEngine as CurrentEngine
from yggdrasil_engine.native_engine import NativeUnleashEngine as Pyo3PocEngine

CALLS = int(os.environ.get("CALLS", 50_000))
THREADS = int(os.environ.get("THREADS", 8))
PER_THREAD = int(os.environ.get("PER_THREAD", 20_000))

STATE = json.dumps(
    {
        "version": 2,
        "segments": [
            {
                "id": 1,
                "name": "launch-countries",
                "constraints": [
                    {"contextName": "country", "operator": "IN", "values": ["us", "de"]}
                ],
            }
        ],
        "features": [
            {
                "name": "simple",
                "enabled": True,
                "type": "release",
                "project": "default",
                "strategies": [{"name": "default"}],
            },
            {
                "name": "rollout",
                "enabled": True,
                "type": "release",
                "project": "default",
                "strategies": [
                    {
                        "name": "flexibleRollout",
                        "parameters": {
                            "rollout": "50",
                            "stickiness": "default",
                            "groupId": "rollout",
                        },
                        "constraints": [
                            {"contextName": "plan", "operator": "IN", "values": ["pro", "ent"]}
                        ],
                    }
                ],
            },
            {
                "name": "complex",
                "enabled": True,
                "type": "release",
                "project": "default",
                "strategies": [
                    {
                        "name": "flexibleRollout",
                        "parameters": {
                            "rollout": "100",
                            "stickiness": "default",
                            "groupId": "complex",
                        },
                        "segments": [1],
                        "constraints": [
                            {
                                "contextName": "email",
                                "operator": "STR_ENDS_WITH",
                                "values": ["@example.com"],
                                "caseInsensitive": True,
                            },
                            {"contextName": "age", "operator": "NUM_GTE", "value": "18"},
                            {"contextName": "appVersion", "operator": "SEMVER_GT", "value": "1.2.0"},
                            {
                                "contextName": "plan",
                                "operator": "IN",
                                "values": ["free"],
                                "inverted": True,
                            },
                            {"contextName": "userId", "operator": "STR_CONTAINS", "values": ["1"]},
                        ],
                    }
                ],
            },
        ],
    }
)

STANDARD_CONTEXT = {
    "userId": "12345",
    "sessionId": "sess-1",
    "appName": "bench",
    "environment": "prod",
    "properties": {"plan": "pro", "tier": "gold", "country": "us", "region": "eu-west-1"},
}
COMPLEX_CONTEXT = {
    **STANDARD_CONTEXT,
    "properties": {
        **STANDARD_CONTEXT["properties"],
        "email": "Jane@Example.com",
        "age": "30",
        "appVersion": "1.4.2",
        "team": "payments",
        "device": "ios",
        "locale": "en-US",
    },
}
FLAGS = ["simple", "rollout", "complex"]
CONTEXTS = {
    "minimal": {"userId": "12345"},
    "standard": STANDARD_CONTEXT,
    "complex": COMPLEX_CONTEXT,
}

def ns_per_call(fn, calls=CALLS):
    for _ in range(min(calls, 5_000)):
        fn()
    start = time.perf_counter_ns()
    for _ in range(calls):
        fn()
    return (time.perf_counter_ns() - start) / calls


def make_engines():
    current, reporter, pyo3_poc = CurrentEngine(), ReporterEngine(), Pyo3PocEngine()
    pyo3_pythonize = Pyo3PocEngine(context_reader="pythonize")
    for engine in (current, reporter, pyo3_poc, pyo3_pythonize):
        engine.take_state(STATE)
    return current, reporter, pyo3_poc, pyo3_pythonize



def reporter_is_enabled(reporter, flag, context):
    enabled = reporter.is_enabled(flag, context)
    reporter.should_emit_impression_event(flag)
    reporter.count_toggle(flag, bool(enabled))
    return enabled


def threaded(fn):
    latencies = [[] for _ in range(THREADS)]

    def worker(index):
        for _ in range(PER_THREAD):
            start = time.perf_counter_ns()
            fn()
            latencies[index].append(time.perf_counter_ns() - start)

    threads = [threading.Thread(target=worker, args=(i,)) for i in range(THREADS)]
    start = time.perf_counter_ns()
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    wall = time.perf_counter_ns() - start
    flat = sorted(latency for per_thread in latencies for latency in per_thread)
    return {
        "p50": flat[len(flat) // 2],
        "p99": flat[int(len(flat) * 0.99)],
        "ops": THREADS * PER_THREAD / (wall / 1e9),
    }


def main():
    current, reporter, pyo3_poc, pyo3_pythonize = make_engines()

    print(f"python {sys.version.split()[0]}, {CALLS:,} calls per cell")
    print(f"current:  {sys.modules['yggdrasil_engine'].__file__}")
    print(f"reporter: {sys.modules['ct_unleash_engine'].__file__}")
    print(f"pyo3_poc: {sys.modules['yggdrasil_native'].__file__}")
    if hasattr(os, "getloadavg"):
        print(f"load avg {os.getloadavg()}")

    print("\nis_enabled, single thread (ns per call)")
    header = (
        f"{'flag':8s} {'context':8s} {'current':>10s} {'reporter':>10s} {'pyo3_poc':>10s}"
        f" {'pyo3_pythonize':>15s} {'pyo3_poc vs current':>20s}"
        f" {'pyo3_pythonize vs current':>26s}"
    )
    print(header)
    print("-" * len(header))
    for flag in FLAGS:
        for context_name, context in CONTEXTS.items():
            current_ns = ns_per_call(lambda: current.is_enabled(flag, context))
            reporter_ns = ns_per_call(lambda: reporter_is_enabled(reporter, flag, context))
            pyo3_poc_ns = ns_per_call(lambda: pyo3_poc.is_enabled(flag, context))
            pythonize_ns = ns_per_call(lambda: pyo3_pythonize.is_enabled(flag, context))
            print(
                f"{flag:8s} {context_name:8s} {current_ns:>10,.0f} {reporter_ns:>10,.0f}"
                f" {pyo3_poc_ns:>10,.0f} {pythonize_ns:>15,.0f}"
                f" {current_ns / pyo3_poc_ns:>19.1f}x"
                f" {current_ns / pythonize_ns:>25.1f}x"
            )

    print(f"\nis_enabled, {THREADS} threads, rollout flag + standard context")
    for name, fn in (
        ("current", lambda: current.is_enabled("rollout", STANDARD_CONTEXT)),
        ("reporter", lambda: reporter_is_enabled(reporter, "rollout", STANDARD_CONTEXT)),
        ("pyo3_poc", lambda: pyo3_poc.is_enabled("rollout", STANDARD_CONTEXT)),
        ("pyo3_pythonize", lambda: pyo3_pythonize.is_enabled("rollout", STANDARD_CONTEXT)),
    ):
        result = threaded(fn)
        print(
            f"  {name:15s} p50 {result['p50']:>10,} ns   p99 {result['p99']:>12,} ns"
            f"   {result['ops']:>10,.0f} ops/s"
        )


if __name__ == "__main__":
    main()
