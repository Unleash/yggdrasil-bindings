use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard};

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use unleash_yggdrasil::state::EnrichedContext;
use unleash_yggdrasil::{Context, EngineState, UpdateMessage};

const CONTEXT_FIELDS: [&str; 6] = [
    "userId",
    "sessionId",
    "environment",
    "appName",
    "currentTime",
    "remoteAddress",
];

#[pyclass(frozen)]
struct NativeEngine {
    state: Mutex<EngineState>,
}

impl NativeEngine {
    fn lock(&self) -> MutexGuard<'_, EngineState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Evaluates, looks up the impression flag and, with `count`, counts a
    /// known toggle, under one lock. Shared by both context readers.
    fn check(
        &self,
        name: &str,
        context: &Context,
        custom_results: Option<&HashMap<String, bool>>,
        count: bool,
    ) -> (Option<bool>, bool) {
        let enriched = EnrichedContext::from(context, name, custom_results);
        let engine = self.lock();
        let enabled = engine.check_enabled(&enriched);
        let impression = engine.should_emit_impression_event(name);
        if let (true, Some(enabled)) = (count, enabled) {
            engine.count_toggle(name, enabled);
        }
        (enabled, impression)
    }
}
fn guard<T>(action: impl FnOnce() -> PyResult<T>) -> PyResult<T> {
    panic::catch_unwind(AssertUnwindSafe(action)).unwrap_or_else(|_| {
        Err(PyValueError::new_err(
            "Engine panicked while processing the request",
        ))
    })
}

#[pymethods]
impl NativeEngine {
    #[new]
    fn new() -> Self {
        NativeEngine {
            state: Mutex::new(EngineState::default()),
        }
    }

    fn take_state(&self, json: &str) -> PyResult<Option<String>> {
        guard(|| {
            let message: UpdateMessage = serde_json::from_str(json)
                .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {e}")))?;
            let warnings = self.lock().take_state(message);
            Ok(warnings.map(|warnings| {
                warnings
                    .iter()
                    .map(|w| format!("{}: {}", w.toggle_name, w.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            }))
        })
    }

    #[pyo3(signature = (name, context, custom_results=None, count=true))]
    fn check_enabled(
        &self,
        name: &str,
        context: &Bound<'_, PyDict>,
        custom_results: Option<HashMap<String, bool>>,
        count: bool,
    ) -> PyResult<(Option<bool>, bool)> {
        guard(|| {
            let context = context_from_dict(context)?;
            Ok(self.check(name, &context, custom_results.as_ref(), count))
        })
    }

    /// Same as `check_enabled`, but reads the context with `pythonize`, through
    /// `Context`'s own deserializer (the JSON path's rules, `Context::from_map`)
    /// instead of `context_from_dict`. For comparing the two readers.
    #[pyo3(signature = (name, context, custom_results=None, count=true))]
    fn check_enabled_pythonize(
        &self,
        name: &str,
        context: &Bound<'_, PyAny>,
        custom_results: Option<HashMap<String, bool>>,
        count: bool,
    ) -> PyResult<(Option<bool>, bool)> {
        guard(|| {
            let context: Context = pythonize::depythonize(context)?;
            Ok(self.check(name, &context, custom_results.as_ref(), count))
        })
    }

    fn count_toggle(&self, name: &str, enabled: bool) {
        self.lock().count_toggle(name, enabled);
    }
}

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

    let mut fields: [Option<String>; 6] = Default::default();
    for (slot, key) in fields.iter_mut().zip(CONTEXT_FIELDS) {
        *slot = match dict.get_item(key)? {
            Some(value) => value_to_string(&value)?,
            None => properties.remove(key),
        };
    }

    for (key, value) in dict.iter() {
        let key = key_to_string(&key)?;
        if key == "properties" || CONTEXT_FIELDS.contains(&key.as_str()) {
            continue;
        }
        if let Ok(value) = value.cast::<PyString>() {
            properties.insert(key, value.to_cow()?.into_owned());
        } else {
            value_to_string(&value)?;
        }
    }

    let [user_id, session_id, environment, app_name, current_time, remote_address] = fields;
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

fn value_to_string(value: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
    if let Ok(value) = value.cast::<PyString>() {
        return Ok(Some(value.to_cow()?.into_owned()));
    }
    if value.is_instance_of::<PyBool>() {
        return Ok(Some(value.is_truthy()?.to_string()));
    }
    if let Ok(value) = value.cast::<PyInt>() {
        return Ok(Some(value.to_string()));
    }
    if let Ok(value) = value.cast::<PyFloat>() {
        return serde_json::Number::from_f64(value.value())
            .map(|n| Some(n.to_string()))
            .ok_or_else(|| PyValueError::new_err("Context contains a non-finite number"));
    }
    check_encodable(value)?;
    Ok(None)
}

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
