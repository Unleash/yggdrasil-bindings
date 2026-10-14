//! PyO3 engine for Python, built temporarily next to the ctypes engine in
//! `yggdrasil_engine.engine`.
//!
//! reads the context from a Python dict and exposes the engine to Python. The engine logic itself is in `engine`

pub mod engine;

use std::borrow::Cow;
use std::collections::HashMap;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use unleash_yggdrasil::{Context, ExtendedVariantDef, CORE_VERSION};

use crate::engine::{guard, Engine, EngineError};

impl From<EngineError> for PyErr {
    fn from(error: EngineError) -> Self {
        PyValueError::new_err(error.to_string())
    }
}

/// A variant as `NativeEngine.get_variant` hands it to Python:
/// `(name, payload, enabled, feature_enabled)`
type VariantTuple<'py> = (String, Option<Bound<'py, PyDict>>, bool, bool);

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
    /// Resolves a toggle's variant and looks up whether it requires an
    /// impression event, without recording anything. Returns
    /// `(variant, requires_impression_event)`, with `variant` `None` for an
    /// unknown toggle, otherwise `(name, payload, enabled, feature_enabled)`
    /// and `payload` `None` or `{"type": ..., "value": ...}`
    #[pyo3(signature = (toggle_name, context, custom_strategy_results=None))]
    fn check_variant<'py>(
        &self,
        toggle_name: &str,
        context: &Bound<'py, PyDict>,
        custom_strategy_results: Option<HashMap<String, bool>>,
    ) -> PyResult<(Option<VariantTuple<'py>>, bool)> {
        guard(|| {
            let py = context.py();
            let context = context_from_dict(context)?;
            let (variant, impression) = self.engine.check_variant(
                toggle_name,
                &context,
                custom_strategy_results.as_ref(),
            )?;
            Ok((variant_to_tuple(py, variant)?, impression))
        })
    }

    /// Like `check_variant`, and also records the toggle and the variant in
    /// the metrics; an unknown toggle is recorded as the disabled variant
    #[pyo3(signature = (toggle_name, context, custom_strategy_results=None))]
    fn get_variant<'py>(
        &self,
        toggle_name: &str,
        context: &Bound<'py, PyDict>,
        custom_strategy_results: Option<HashMap<String, bool>>,
    ) -> PyResult<(Option<VariantTuple<'py>>, bool)> {
        guard(|| {
            let py = context.py();
            let context = context_from_dict(context)?;
            let (variant, impression) =
                self.engine
                    .get_variant(toggle_name, &context, custom_strategy_results.as_ref())?;
            Ok((variant_to_tuple(py, variant)?, impression))
        })
    }

    fn count_toggle(&self, toggle_name: &str, enabled: bool) -> PyResult<()> {
        Ok(self.engine.count_toggle(toggle_name, enabled)?)
    }

    fn get_metrics(&self) -> PyResult<Option<String>> {
        let Some(bucket) = self.engine.get_metrics()? else {
            return Ok(None);
        };
        serde_json::to_string(&bucket)
            .map(Some)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn get_state(&self) -> PyResult<String> {
        serde_json::to_string(&self.engine.get_state()?)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn list_known_toggles(&self) -> PyResult<Vec<(String, String, Option<String>)>> {
        Ok(self
            .engine
            .list_known_toggles()?
            .into_iter()
            .map(|toggle| (toggle.name, toggle.project, toggle.feature_type))
            .collect())
    }

    fn define_counter(&self, name: &str, help: &str) -> PyResult<()> {
        Ok(self.engine.define_counter(name, help)?)
    }

    fn inc_counter(
        &self,
        name: &str,
        value: i64,
        labels: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        Ok(self.engine.inc_counter(name, value, labels.as_ref())?)
    }

    fn define_gauge(&self, name: &str, help: &str) -> PyResult<()> {
        Ok(self.engine.define_gauge(name, help)?)
    }

    fn set_gauge(
        &self,
        name: &str,
        value: f64,
        labels: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        Ok(self.engine.set_gauge(name, value, labels.as_ref())?)
    }

    fn define_histogram(&self, name: &str, help: &str, buckets: Option<Vec<f64>>) -> PyResult<()> {
        Ok(self
            .engine
            .define_histogram(name, help, buckets.unwrap_or_default())?)
    }

    fn observe_histogram(
        &self,
        name: &str,
        value: f64,
        labels: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        Ok(self
            .engine
            .observe_histogram(name, value, labels.as_ref())?)
    }

    fn collect_impact_metrics(&self) -> PyResult<String> {
        serde_json::to_string(&self.engine.collect_impact_metrics()?)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

fn variant_to_tuple<'py>(
    py: Python<'py>,
    variant: Option<ExtendedVariantDef>,
) -> PyResult<Option<VariantTuple<'py>>> {
    let Some(variant) = variant else {
        return Ok(None);
    };
    let payload = match variant.payload {
        Some(payload) => {
            let dict = PyDict::new(py);
            dict.set_item(intern!(py, "type"), payload.payload_type)?;
            dict.set_item(intern!(py, "value"), payload.value)?;
            Some(dict)
        }
        None => None,
    };
    Ok(Some((
        variant.name,
        payload,
        variant.enabled,
        variant.feature_enabled,
    )))
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
    let py = dict.py();
    let nested;
    let dict = match dict.get_item(intern!(py, "context"))? {
        Some(inner) => {
            nested = inner
                .cast_into::<PyDict>()
                .map_err(|_| PyValueError::new_err("Expected 'context' to be an object"))?;
            &nested
        }
        None => dict,
    };

    let mut properties = HashMap::new();
    if let Some(raw_properties) = dict.get_item(intern!(py, "properties"))? {
        if let Ok(raw_properties) = raw_properties.cast::<PyDict>() {
            properties.reserve(raw_properties.len());
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
    let mut fields: [(&str, &Bound<'_, PyString>, &mut Option<String>); 6] = [
        ("userId", intern!(py, "userId"), &mut user_id),
        ("sessionId", intern!(py, "sessionId"), &mut session_id),
        ("environment", intern!(py, "environment"), &mut environment),
        ("appName", intern!(py, "appName"), &mut app_name),
        ("currentTime", intern!(py, "currentTime"), &mut current_time),
        (
            "remoteAddress",
            intern!(py, "remoteAddress"),
            &mut remote_address,
        ),
    ];

    for (key, py_key, slot) in fields.iter_mut() {
        **slot = match dict.get_item(*py_key)? {
            Some(value) => value_to_string(&value)?,
            None => properties.remove(*key),
        };
    }

    // Unknown top-level strings move into properties; other values are dropped
    for (key, value) in dict.iter() {
        let key = match key.cast::<PyString>() {
            Ok(key) => key.to_cow()?,
            Err(_) => Cow::Owned(key_to_string(&key)?),
        };
        if key == "properties" || fields.iter().any(|(name, _, _)| *name == key) {
            continue;
        }
        if let Ok(value) = value.cast::<PyString>() {
            properties.insert(key.into_owned(), value.to_cow()?.into_owned());
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

/// Test helper, only with the `test-helpers` feature: the `Context` our reader
/// makes from a dict, as JSON. With `context_from_json`, lets the tests check
/// that `context_from_dict` keeps the same rules as unleash-types (it copies
/// them). `is_enabled` only returns the answer, not the `Context`
/// not used by the engine, not in the shipped wheel
#[cfg(feature = "test-helpers")]
#[pyfunction]
#[pyo3(name = "context_from_dict")]
fn context_from_dict_as_json(context: &Bound<'_, PyDict>) -> PyResult<String> {
    let context = context_from_dict(context)?;
    serde_json::to_string(&context).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Test helper, only with the `test-helpers` feature: the `Context`
/// unleash-types makes from JSON (its own rules, `Context::from_map`), as JSON.
#[cfg(feature = "test-helpers")]
#[pyfunction]
#[pyo3(name = "context_from_json")]
fn context_from_json_as_json(json: &str) -> PyResult<String> {
    let context: Context =
        serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    serde_json::to_string(&context).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
fn core_version() -> &'static str {
    CORE_VERSION
}

#[pymodule]
fn yggdrasil_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeEngine>()?;
    m.add_function(wrap_pyfunction!(core_version, m)?)?;
    #[cfg(feature = "test-helpers")]
    {
        m.add_function(wrap_pyfunction!(context_from_dict_as_json, m)?)?;
        m.add_function(wrap_pyfunction!(context_from_json_as_json, m)?)?;
    }
    Ok(())
}
