//! PyO3 engine for Python, built temporarily next to the ctypes engine in
//! `yggdrasil_engine.engine`.
//!
//! reads the context from a Python dict and exposes the engine to Python. The engine logic itself is in `engine`

pub mod engine;

use std::collections::HashMap;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use unleash_yggdrasil::Context;

use crate::engine::{guard, Engine, EngineError};

impl From<EngineError> for PyErr {
    fn from(error: EngineError) -> Self {
        PyValueError::new_err(error.to_string())
    }
}

#[pyclass(frozen)]
struct NativeEngine {
    engine: Engine,
}

#[pymethods]
impl NativeEngine {
    #[new]
    fn new() -> Self {
        NativeEngine {
            engine: Engine::default(),
        }
    }

    fn take_state(&self, state_json: &str) -> PyResult<Option<String>> {
        let warnings = self.engine.take_state(state_json)?;
        Ok(warnings.filter(|w| !w.is_empty()).map(|warnings| {
            warnings
                .iter()
                .map(|w| format!("{}: {}", w.toggle_name, w.message))
                .collect::<Vec<_>>()
                .join("\n")
        }))
    }

    /// Evaluates a toggle, looks up whether it requires an impression event,
    /// and records a known toggle in the metrics. Returns
    /// `(enabled, requires_impression_event)`, with `enabled` `None` for an
    /// unknown toggle, which isn't recorded (see `count_toggle`)
    #[pyo3(signature = (toggle_name, context, custom_strategy_results=None))]
    fn is_enabled(
        &self,
        toggle_name: &str,
        context: &Bound<'_, PyDict>,
        custom_strategy_results: Option<HashMap<String, bool>>,
    ) -> PyResult<(Option<bool>, bool)> {
        guard(|| {
            let context = context_from_dict(context)?;
            Ok(self
                .engine
                .is_enabled(toggle_name, &context, custom_strategy_results.as_ref())?)
        })
    }
    fn count_toggle(&self, toggle_name: &str, enabled: bool) -> PyResult<()> {
        Ok(self.engine.count_toggle(toggle_name, enabled)?)
    }
}

/// Reads a context dict into the core's `Context`, with the same rules as the
/// ctypes engine's path: `json.dumps`, then `Context`'s JSON parsing
/// (`Context::from_map` in unleash-types). Both engines must see the same
/// context.
///
/// These rules are duplicated from unleash-types for now. The plan is to remove the duplication in
/// unleash-types: a public constructor that builds a `Context` from simple
/// key/value pairs, used by both `from_map` and this reader, so the rules live
/// in one place.
fn context_from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Context> {
    let nested;
    let dict = match dict.get_item("context")? {
        Some(inner) => {
            nested = inner
                .cast_into::<PyDict>()
                .map_err(|_| PyValueError::new_err("Expected 'context' to be an object"))?;
            &nested
        }
        None => dict,
    };

    let mut properties = HashMap::new();
    if let Some(raw_properties) = dict.get_item("properties")? {
        if let Ok(raw_properties) = raw_properties.cast::<PyDict>() {
            for (key, value) in raw_properties.iter() {
                let key = key_to_string(&key)?;
                if let Some(value) = value_to_string(&value)? {
                    properties.insert(key, value);
                }
            }
        } else {
            // not a dict, so ignored, but it must still be valid JSON
            value_to_string(&raw_properties)?;
        }
    }

    let mut user_id = None;
    let mut session_id = None;
    let mut environment = None;
    let mut app_name = None;
    let mut current_time = None;
    let mut remote_address = None;
    let mut fields: [(&str, &mut Option<String>); 6] = [
        ("userId", &mut user_id),
        ("sessionId", &mut session_id),
        ("environment", &mut environment),
        ("appName", &mut app_name),
        ("currentTime", &mut current_time),
        ("remoteAddress", &mut remote_address),
    ];

    for (key, slot) in fields.iter_mut() {
        **slot = match dict.get_item(*key)? {
            Some(value) => value_to_string(&value)?,
            None => properties.remove(*key),
        };
    }

    // Unknown top-level strings move into properties; other values are dropped
    for (key, value) in dict.iter() {
        let key = key_to_string(&key)?;
        if key == "properties" || fields.iter().any(|(name, _)| *name == key) {
            continue;
        }
        if let Ok(value) = value.cast::<PyString>() {
            properties.insert(key, value.to_cow()?.into_owned());
        } else {
            value_to_string(&value)?;
        }
    }

    Ok(Context {
        user_id,
        session_id,
        environment,
        app_name,
        current_time,
        remote_address,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
    })
}

/// Strings, numbers and bools become strings, as in the core's JSON parsing;
/// anything else has no usable value
fn value_to_string(value: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
    if let Ok(value) = value.cast::<PyString>() {
        return Ok(Some(value.to_cow()?.into_owned()));
    }
    // Before PyInt: bool is a subclass of int
    if value.is_instance_of::<PyBool>() {
        return Ok(Some(value.is_truthy()?.to_string()));
    }
    if let Ok(value) = value.cast::<PyInt>() {
        return Ok(Some(value.to_string()));
    }
    if let Ok(value) = value.cast::<PyFloat>() {
        // Formatted like a JSON number, and NaN/infinity rejected like the
        // JSON parser does
        return serde_json::Number::from_f64(value.value())
            .map(|number| Some(number.to_string()))
            .ok_or_else(|| PyValueError::new_err("Context contains a non-finite number"));
    }
    check_encodable(value)?;
    Ok(None)
}

/// Dict keys become strings the way `json.dumps` turns them into JSON keys
fn key_to_string(key: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(key) = key.cast::<PyString>() {
        return Ok(key.to_cow()?.into_owned());
    }
    if key.is_none() {
        return Ok("null".to_owned());
    }
    match value_to_string(key)? {
        Some(key) => Ok(key),
        None => Err(PyTypeError::new_err(
            "Context keys must be str, int, float, bool or None",
        )),
    }
}

/// Fails for values `json.dumps` would refuse, so both engines fail on the
/// same contexts
fn check_encodable(value: &Bound<'_, PyAny>) -> PyResult<()> {
    if value.is_none()
        || value.is_instance_of::<PyString>()
        || value.is_instance_of::<PyBool>()
        || value.is_instance_of::<PyInt>()
        || value.is_instance_of::<PyFloat>()
        || value.is_instance_of::<PyList>()
        || value.is_instance_of::<PyTuple>()
        || value.is_instance_of::<PyDict>()
    {
        Ok(())
    } else {
        Err(PyTypeError::new_err(format!(
            "Context value of type {} is not JSON serializable",
            value.get_type().name()?
        )))
    }
}

#[pymodule]
fn yggdrasil_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeEngine>()?;
    Ok(())
}
