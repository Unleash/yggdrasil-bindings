## The drift guard for the context reader: `context_from_dict` (lib.rs) copies
## unleash-types' context rules (`Context::from_map`, used on the JSON path) to
## read the dict directly. Each case reads a context both ways and expects the
## same `Context`, or both ways to fail:
##
##   context_from_dict(ctx)  ==  unleash-types' parsing of json.dumps(ctx)
##
## Needs the module built with the `test-helpers` cargo feature (see README.md).

import json
import math

import pytest

try:
    from yggdrasil_engine.yggdrasil_native import (  # type: ignore
        context_from_dict,
        context_from_json,
    )
except ImportError as error:
    raise ImportError(
        "the context reader tests need the module built with the test-helpers "
        "feature: `maturin develop --features test-helpers`"
    ) from error


class Unserializable:
    pass


def _read_with_pyo3_reader(context):
    try:
        return json.loads(context_from_dict(context)), None
    except Exception as error:
        return None, error


def _read_with_unleash_types(context):
    try:
        return json.loads(context_from_json(json.dumps(context))), None
    except Exception as error:
        return None, error


def _assert_same(context):
    pyo3_context, pyo3_error = _read_with_pyo3_reader(context)
    unleash_types_context, unleash_types_error = _read_with_unleash_types(context)
    assert (pyo3_error is None) == (unleash_types_error is None), (
        f"PyO3 reader: {pyo3_error!r}, unleash-types: {unleash_types_error!r}"
    )
    assert pyo3_context == unleash_types_context


CASES = {
    ## fields
    "empty": {},
    "all fields at the top level": {
        "userId": "u1",
        "sessionId": "s1",
        "environment": "prod",
        "appName": "app",
        "currentTime": "2026-10-12T10:00:00Z",
        "remoteAddress": "10.0.0.1",
    },
    "fields only in properties": {
        "properties": {"userId": "u1", "sessionId": "s1", "remoteAddress": "10.0.0.1"}
    },
    "field at the top level and in properties": {
        "userId": "top",
        "properties": {"userId": "nested"},
    },
    "field as a number": {"userId": 123, "sessionId": 1.5},
    "field as a bool": {"userId": True},
    "field as None does not fall back to properties": {
        "userId": None,
        "properties": {"userId": "nested"},
    },
    "field as a list does not fall back to properties": {
        "userId": ["a"],
        "properties": {"userId": "nested"},
    },
    "field as a dict does not fall back to properties": {
        "userId": {"a": 1},
        "properties": {"userId": "nested"},
    },
    "field in properties as a number": {"properties": {"userId": 42}},
    ## top-level leftovers
    "leftover string kept": {"region": "eu"},
    "leftover number dropped": {"age": 30},
    "leftover float dropped": {"ratio": 0.5},
    "leftover bool dropped": {"beta": True},
    "leftover None dropped": {"region": None},
    "leftover list dropped": {"tags": ["a", "b"]},
    "leftover dict dropped": {"nested": {"a": "b"}},
    "leftover string overrides properties": {
        "region": "top",
        "properties": {"region": "nested"},
    },
    "leftover number keeps properties": {
        "region": 5,
        "properties": {"region": "nested"},
    },
    ## properties
    "properties of every kind": {
        "properties": {
            "s": "text",
            "i": 7,
            "f": 2.5,
            "b": False,
            "n": None,
            "l": [1],
            "d": {"x": 1},
        }
    },
    "properties as a string": {"properties": "not a dict"},
    "properties as a list": {"properties": ["a"]},
    "properties as None": {"properties": None},
    "empty properties": {"properties": {}},
    ## nested context
    "nested context": {"context": {"userId": "u1", "properties": {"plan": "pro"}}},
    "nested context ignores the outer keys": {
        "userId": "outer",
        "context": {"userId": "inner"},
    },
    "context that is not a dict": {"context": "nope"},
    ## keys json.dumps converts
    "int key": {1: "one"},
    "int key in properties": {"properties": {1: "one", 2: 2}},
    "key that becomes a field name": {"properties": {}, "userId": "a"},
    ## numbers
    "float formatting": {"properties": {"a": 1.0, "b": 0.1, "c": -0.0, "d": 1e-7}},
    "large float": {"properties": {"a": 1e20}},
    "negative int": {"properties": {"a": -5}},
    "int at the 64-bit limits": {"properties": {"a": 2**63 - 1, "b": -(2**63)}},
    "NaN": {"properties": {"a": math.nan}},
    "infinity": {"properties": {"a": math.inf}},
    "NaN at the top level": {"ratio": math.nan},
    ## strings
    "unicode": {"userId": "ü→🚀", "properties": {"名前": "値"}},
    "empty strings": {"userId": "", "properties": {"": ""}},
    ## values json.dumps refuses
    "set value": {"properties": {"a": {1, 2}}},
    "object value": {"region": Unserializable()},
    "tuple value": {"properties": {"a": (1, 2)}},
}


@pytest.mark.parametrize("context", CASES.values(), ids=CASES.keys())
def test_context_reader_matches_unleash_types(context):
    _assert_same(context)
