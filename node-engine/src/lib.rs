#![deny(clippy::all)]

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tsify::{Ts, Tsify};
use unleash_yggdrasil::{
    impact_metrics::{BucketMetricOptions, CollectedMetric, MetricOptions},
    state::{EnrichedContext, ExternalResultsRef, PropertiesRef},
    ClientFeatures, EngineState, ExtendedVariantDef, MetricBucket, ToggleDefinition, UpdateMessage,
    CORE_VERSION, KNOWN_STRATEGIES, SUPPORTED_SPEC_VERSION,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmInterface {
    inner: Option<EngineState>,
}

#[derive(Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct EnabledResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    pub impression_data: bool,
}

#[derive(Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct VariantResult {
    pub variant: Option<ExtendedVariantDef>,
    pub impression_data: bool,
}

#[derive(Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct KnownFeatures {
    features: Vec<ToggleDefinition>,
}

#[derive(Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[tsify(hashmap_as_object)]
pub struct MetricsResult {
    bucket: Option<MetricBucket>,
    impact_metrics: Vec<CollectedMetric>,
}

#[derive(Serialize, Tsify)]
#[serde(transparent)]
#[tsify(missing_as_null)]
pub struct StateWarnings(Option<String>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParsedContext {
    toggle_name: String,
    properties: Option<HashMap<String, Value>>,
    external_results: Option<HashMap<String, bool>>,
    #[serde(flatten)]
    fields: HashMap<String, Value>,
}

struct OwnedContext {
    toggle_name: String,
    fields: HashMap<String, String>,
    properties: HashMap<String, String>,
    external_results: Option<HashMap<String, bool>>,
}

fn context_string(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value),
        value => Some(value.to_string()),
    }
}

impl ParsedContext {
    fn normalize(self) -> OwnedContext {
        let mut fields = HashMap::new();
        let mut properties = HashMap::new();

        for (key, value) in self.fields {
            if let Some(value) = context_string(value) {
                match key.as_str() {
                    "userId" | "sessionId" | "environment" | "appName" | "currentTime"
                    | "remoteAddress" | "runtimeHostname" => {
                        fields.insert(key, value);
                    }
                    _ => {
                        properties.insert(key, value);
                    }
                }
            }
        }

        if let Some(explicit) = self.properties {
            for (key, value) in explicit {
                match context_string(value) {
                    Some(value) => {
                        properties.insert(key, value);
                    }
                    None => {
                        properties.remove(&key);
                    }
                }
            }
        }

        OwnedContext {
            toggle_name: self.toggle_name,
            fields,
            properties,
            external_results: self.external_results,
        }
    }
}

impl OwnedContext {
    fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }

    fn enriched(&self) -> EnrichedContext<'_> {
        EnrichedContext {
            user_id: self.field("userId"),
            session_id: self.field("sessionId"),
            environment: self.field("environment"),
            app_name: self.field("appName"),
            current_time: self.field("currentTime"),
            remote_address: self.field("remoteAddress"),
            runtime_hostname: self.field("runtimeHostname"),
            toggle_name: &self.toggle_name,
            properties: Some(PropertiesRef::Strings(&self.properties)),
            external_results: self
                .external_results
                .as_ref()
                .map(ExternalResultsRef::Strings),
        }
    }
}

impl Default for WasmInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_name = getSpecVersion)]
pub fn get_spec_version() -> String {
    SUPPORTED_SPEC_VERSION.to_string()
}

#[wasm_bindgen(js_name = getYggdrasilVersion)]
pub fn get_yggdrasil_version() -> String {
    CORE_VERSION.to_string()
}

#[wasm_bindgen(js_name = getSupportedStrategies)]
pub fn get_supported_strategies() -> Vec<String> {
    KNOWN_STRATEGIES.iter().map(|s| s.to_string()).collect()
}

#[wasm_bindgen]
impl WasmInterface {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Some(EngineState::default()),
        }
    }

    #[wasm_bindgen]
    pub fn destroy(&mut self) {
        self.inner = None;
    }

    #[wasm_bindgen(js_name = takeState)]
    pub fn take_state(&mut self, raw_state: &str) -> Result<Ts<StateWarnings>, JsError> {
        let engine = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let client_features = serde_json::from_str::<UpdateMessage>(raw_state)
            .map_err(|err| JsError::new(&format!("Failed to parse client features: {err}")))?;

        let warnings = engine.take_state(client_features);

        let warnings = warnings.filter(|w| !w.is_empty()).map(|warnings| {
            warnings
                .iter()
                .map(|w| format!("{}: {}", w.toggle_name, w.message))
                .collect::<Vec<_>>()
                .join("\n")
        });

        Ok(StateWarnings(warnings).into_ts()?)
    }

    #[wasm_bindgen(js_name = isEnabled)]
    pub fn is_enabled(&self, enriched_context: &str) -> Result<Ts<EnabledResult>, JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let message: ParsedContext = serde_json::from_str(enriched_context)
            .map_err(|err| JsError::new(&format!("Failed to parse enriched context: {err}")))?;

        let message = message.normalize();
        let context = message.enriched();

        let is_enabled = engine.check_enabled(&context);

        if let Some(is_enabled) = is_enabled {
            engine.count_toggle(context.toggle_name, is_enabled);
        }

        Ok(EnabledResult {
            enabled: is_enabled,
            impression_data: engine.should_emit_impression_event(context.toggle_name),
        }
        .into_ts()?)
    }

    #[wasm_bindgen(js_name = getVariant)]
    pub fn get_variant(&self, enriched_context: &str) -> Result<Ts<VariantResult>, JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let message: ParsedContext = serde_json::from_str(enriched_context)
            .map_err(|err| JsError::new(&format!("Failed to parse enriched context: {err}")))?;

        let message = message.normalize();
        let context = message.enriched();

        let enabled = engine.check_enabled(&context).unwrap_or_default();
        let variant = engine
            .check_variant(&context)
            .map(|variant| variant.to_enriched_response(enabled));

        if let Some(variant) = &variant {
            // if the variant exists the SDK will short circuit return  and expect
            // the engine to have recorded the toggle count as well. Unlike the
            // is_enabled call, we can know the state of the fallback value here
            // for the enabled state - it's always false because the user cannot define it
            engine.count_toggle(context.toggle_name, enabled);
            engine.count_variant(context.toggle_name, &variant.name);
        }

        let variant_result = VariantResult {
            variant,
            impression_data: engine.should_emit_impression_event(context.toggle_name),
        };

        Ok(variant_result.into_ts()?)
    }

    #[wasm_bindgen(js_name = checkVariant)]
    pub fn check_variant(&self, enriched_context: &str) -> Result<Ts<VariantResult>, JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let message: ParsedContext = serde_json::from_str(enriched_context)
            .map_err(|err| JsError::new(&format!("Failed to parse enriched context: {err}")))?;

        let message = message.normalize();
        let context = message.enriched();

        let enabled = engine.check_enabled(&context).unwrap_or_default();
        let variant = engine
            .check_variant(&context)
            .map(|variant| variant.to_enriched_response(enabled));

        let variant_result = VariantResult {
            variant,
            impression_data: engine.should_emit_impression_event(context.toggle_name),
        };

        Ok(variant_result.into_ts()?)
    }

    #[wasm_bindgen(js_name = count)]
    pub fn count(&self, toggle_name: &str, is_enabled: bool) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        engine.count_toggle(toggle_name, is_enabled);
        Ok(())
    }

    #[wasm_bindgen(js_name = countVariant)]
    pub fn count_variant(&self, toggle_name: &str, variant_name: &str) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        engine.count_variant(toggle_name, variant_name);
        Ok(())
    }

    #[wasm_bindgen(js_name = getMetrics)]
    pub fn get_metrics(&self) -> Result<Ts<MetricsResult>, JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let metrics = MetricsResult {
            bucket: engine.get_metrics(Utc::now()),
            impact_metrics: engine.collect_impact_metrics(),
        };
        Ok(metrics.into_ts()?)
    }

    #[wasm_bindgen(js_name = restoreMetrics)]
    pub fn restore_metrics(&self, metrics_result: &str) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let metrics: MetricsResult = serde_json::from_str(metrics_result)
            .map_err(|e| JsError::new(&format!("Failed to parse metrics result: {}", e)))?;

        engine.restore_impact_metrics(metrics.impact_metrics);
        if let Some(metrics_bucket) = metrics.bucket {
            engine.restore_metrics(metrics_bucket);
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = getState)]
    pub fn get_state(&self) -> Result<Ts<ClientFeatures>, JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let state = engine.get_state();
        Ok(state.into_ts()?)
    }

    #[wasm_bindgen(js_name = listFeatures)]
    pub fn list_features(&self) -> Result<Ts<KnownFeatures>, JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let features = KnownFeatures {
            features: engine.list_known_toggles(),
        };
        Ok(features.into_ts()?)
    }

    #[wasm_bindgen(js_name = defineCounter)]
    pub fn define_counter(&self, name: &str, help_text: &str) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let metrics_options = MetricOptions::new(name, help_text);

        engine.define_counter(metrics_options);
        Ok(())
    }

    #[wasm_bindgen(js_name = defineGauge)]
    pub fn define_gauge(&self, name: &str, help_text: &str) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let metrics_options = MetricOptions::new(name, help_text);

        engine.define_gauge(metrics_options);
        Ok(())
    }

    #[wasm_bindgen(js_name = defineHistogram)]
    pub fn define_histogram(
        &self,
        name: &str,
        help_text: &str,
        buckets: Vec<f64>,
    ) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let metrics_options = BucketMetricOptions::new(name, help_text, buckets);

        engine.define_histogram(metrics_options);
        Ok(())
    }

    #[wasm_bindgen(js_name = incrementCounter)]
    pub fn increment_counter(&self, name: &str, value: i64) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        engine.inc_counter_by(name, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = updateGauge)]
    pub fn update_gauge(&self, name: &str, value: f64) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        engine.set_gauge(name, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = observeHistogram)]
    pub fn observe_histogram(&self, name: &str, value: f64) -> Result<(), JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        engine.observe_histogram(name, value);
        Ok(())
    }
}
