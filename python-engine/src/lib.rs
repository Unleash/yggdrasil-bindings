//! PyO3 engine for Python

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::panic::{self, AssertUnwindSafe};

use chrono::Utc;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use unleash_yggdrasil::impact_metrics::{BucketMetricOptions, CollectedMetric, MetricOptions};
use unleash_yggdrasil::state::EnrichedContext;
use unleash_yggdrasil::{
    Context, EngineState, ExtendedVariantDef, UpdateMessage, VariantDef, CORE_VERSION,
};

#[derive(Debug)]
enum EngineError {
    InvalidJson(String),
    Panic,
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            EngineError::InvalidJson(message) => write!(f, "Failed to parse JSON: {message}"),
            EngineError::Panic => write!(
                f,
                "Engine panicked while processing the request. Please report this as a bug with the accompanying stack trace if available."
            ),
        }
    }
}

impl From<serde_json::Error> for EngineError {
    fn from(error: serde_json::Error) -> Self {
        EngineError::InvalidJson(error.to_string())
    }
}

impl From<EngineError> for PyErr {
    fn from(error: EngineError) -> Self {
        PyValueError::new_err(error.to_string())
    }
}

/// Turns a panic into a `ValueError`
fn guard<T>(action: impl FnOnce() -> PyResult<T>) -> PyResult<T> {
    panic::catch_unwind(AssertUnwindSafe(action)).unwrap_or_else(|_| Err(EngineError::Panic.into()))
}

type VariantTuple<'py> = (String, Option<Bound<'py, PyDict>>, bool, bool);
type ToggleDefinitionTuple = (String, String, Option<String>, bool);

#[pyclass(frozen)]
struct NativeEngine {
    state: EngineState,
}

#[pymethods]
impl NativeEngine {
    #[new]
    fn new() -> Self {
        NativeEngine {
            state: EngineState::default(),
        }
    }

    fn take_state(&self, py: Python<'_>, state_json: &str) -> PyResult<Option<String>> {
        let warnings = py.detach(|| {
            guard(|| {
                let message: UpdateMessage =
                    serde_json::from_str(state_json).map_err(EngineError::from)?;
                Ok(self.state.take_state(message))
            })
        })?;
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
    /// `(enabled, impression)`, with `enabled` `None` for an
    /// unknown toggle, which isn't recorded
    #[pyo3(signature = (toggle_name, context, custom_strategy_results=None))]
    fn is_enabled(
        &self,
        toggle_name: &str,
        context: &Bound<'_, PyDict>,
        custom_strategy_results: Option<HashMap<String, bool>>,
    ) -> PyResult<(Option<bool>, bool)> {
        guard(|| {
            let context = context_from_dict(context)?;
            let engine = &self.state;
            let enabled = engine.check_enabled(&EnrichedContext::from(
                &context,
                toggle_name,
                custom_strategy_results.as_ref(),
            ));
            let impression = engine.should_emit_impression_event(toggle_name);
            if let Some(enabled) = enabled {
                engine.count_toggle(toggle_name, enabled);
            }
            Ok((enabled, impression))
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
            let (variant, impression) = resolve_variant(
                &self.state,
                toggle_name,
                &context,
                custom_strategy_results.as_ref(),
            );
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
            let engine = &self.state;
            let (variant, impression) = resolve_variant(
                engine,
                toggle_name,
                &context,
                custom_strategy_results.as_ref(),
            );
            let disabled;
            let (feature_enabled, variant_name) = match &variant {
                Some(variant) => (variant.feature_enabled, variant.name.as_str()),
                None => {
                    disabled = VariantDef::default();
                    (false, disabled.name.as_str())
                }
            };
            engine.count_toggle(toggle_name, feature_enabled);
            engine.count_variant(toggle_name, variant_name);
            Ok((variant_to_tuple(py, variant)?, impression))
        })
    }

    fn count_toggle(&self, toggle_name: &str, enabled: bool) -> PyResult<()> {
        guard(|| {
            self.state.count_toggle(toggle_name, enabled);
            Ok(())
        })
    }

    fn get_metrics(&self) -> PyResult<Option<String>> {
        let Some(bucket) = guard(|| Ok(self.state.get_metrics(Utc::now())))? else {
            return Ok(None);
        };
        serde_json::to_string(&bucket)
            .map(Some)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn get_state(&self) -> PyResult<String> {
        serde_json::to_string(&guard(|| Ok(self.state.get_state()))?)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn list_known_toggles(&self) -> PyResult<Vec<ToggleDefinitionTuple>> {
        Ok(guard(|| Ok(self.state.list_known_toggles()))?
            .into_iter()
            .map(|toggle| {
                (
                    toggle.name,
                    toggle.project,
                    toggle.feature_type,
                    toggle.enabled,
                )
            })
            .collect())
    }

    fn define_counter(&self, name: &str, help: &str) -> PyResult<()> {
        guard(|| {
            self.state.define_counter(MetricOptions::new(name, help));
            Ok(())
        })
    }

    fn inc_counter(
        &self,
        name: &str,
        value: i64,
        labels: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        guard(|| {
            match labels {
                Some(labels) if !labels.is_empty() => {
                    self.state.inc_counter_with_labels(name, value, &labels)
                }
                _ => self.state.inc_counter_by(name, value),
            }
            Ok(())
        })
    }

    fn define_gauge(&self, name: &str, help: &str) -> PyResult<()> {
        guard(|| {
            self.state.define_gauge(MetricOptions::new(name, help));
            Ok(())
        })
    }

    fn set_gauge(
        &self,
        name: &str,
        value: f64,
        labels: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        guard(|| {
            match labels {
                Some(labels) if !labels.is_empty() => {
                    self.state.set_gauge_with_labels(name, value, &labels)
                }
                _ => self.state.set_gauge(name, value),
            }
            Ok(())
        })
    }

    fn define_histogram(&self, name: &str, help: &str, buckets: Option<Vec<f64>>) -> PyResult<()> {
        guard(|| {
            self.state.define_histogram(BucketMetricOptions::new(
                name,
                help,
                buckets.unwrap_or_default(),
            ));
            Ok(())
        })
    }

    fn observe_histogram(
        &self,
        name: &str,
        value: f64,
        labels: Option<HashMap<String, String>>,
    ) -> PyResult<()> {
        guard(|| {
            match labels {
                Some(labels) if !labels.is_empty() => self
                    .state
                    .observe_histogram_with_labels(name, value, &labels),
                _ => self.state.observe_histogram(name, value),
            }
            Ok(())
        })
    }

    fn collect_impact_metrics(&self) -> PyResult<String> {
        serde_json::to_string(&guard(|| Ok(self.state.collect_impact_metrics()))?)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn restore_impact_metrics(&self, metrics_json: &str) -> PyResult<()> {
        guard(|| {
            let metrics: Vec<CollectedMetric> =
                serde_json::from_str(metrics_json).map_err(EngineError::from)?;
            self.state.restore_impact_metrics(metrics);
            Ok(())
        })
    }
}

fn resolve_variant(
    engine: &EngineState,
    toggle_name: &str,
    context: &Context,
    custom_strategy_results: Option<&HashMap<String, bool>>,
) -> (Option<ExtendedVariantDef>, bool) {
    let context = EnrichedContext::from(context, toggle_name, custom_strategy_results);
    let variant = engine.check_variant(&context).map(|variant| {
        let enabled = engine.check_enabled(&context).unwrap_or_default();
        variant.to_enriched_response(enabled)
    });
    (variant, engine.should_emit_impression_event(toggle_name))
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

/// Reads a context dict into the core's `Context`, with the same rules as
/// `json.dumps`, then `Context`'s JSON parsing (`Context::from_map` in
/// unleash-types)
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
    // This is to support some legacy behavior in the Python SDK that's hard to remove.
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

/// Strings, numbers and bools become strings; other values give `None`, or
/// an error when `json.dumps` would refuse them
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

/// Test helper, only with the `test-helpers` feature: the `Context` our
/// reader makes from a dict, as JSON. Not used by the engine, not in the
/// shipped wheel
#[cfg(feature = "test-helpers")]
#[pyfunction]
#[pyo3(name = "context_from_dict")]
fn context_from_dict_as_json(context: &Bound<'_, PyDict>) -> PyResult<String> {
    let context = context_from_dict(context)?;
    serde_json::to_string(&context).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Test helper, only with the `test-helpers` feature: the `Context`
/// unleash-types makes from JSON (its own rules, `Context::from_map`), as JSON
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
