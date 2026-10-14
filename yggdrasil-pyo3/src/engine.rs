//! Engine logic copied from the yggdrasilffi shim
//!  engine state behind a lock, panic guard, and the calls `is_enabled` needs
//!
//! only Rust with no PyO3 types, so it can later move into a library shared
//! with the shim to remove duplication

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard};

use chrono::Utc;
use unleash_types::client_features::ClientFeatures;
use unleash_types::client_metrics::MetricBucket;
use unleash_yggdrasil::impact_metrics::{
    BucketMetricOptions, CollectedMetric, MetricLabels, MetricOptions,
};
use unleash_yggdrasil::state::EnrichedContext;
use unleash_yggdrasil::{
    Context, EngineState, EvalWarning, ExtendedVariantDef, ToggleDefinition, UpdateMessage,
    VariantDef,
};

#[derive(Debug)]
pub enum EngineError {
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

#[derive(Default)]
pub struct Engine {
    state: Mutex<EngineState>,
}

impl Engine {
    fn lock(&self) -> MutexGuard<'_, EngineState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn take_state(&self, json: &str) -> Result<Option<Vec<EvalWarning>>, EngineError> {
        guard(|| {
            let message: UpdateMessage =
                serde_json::from_str(json).map_err(|e| EngineError::InvalidJson(e.to_string()))?;
            Ok(self.lock().take_state(message))
        })
    }

    pub fn check_enabled(
        &self,
        toggle_name: &str,
        context: &Context,
        custom_strategy_results: Option<&HashMap<String, bool>>,
    ) -> Result<Option<bool>, EngineError> {
        guard(|| {
            Ok(evaluate(
                &self.lock(),
                toggle_name,
                context,
                custom_strategy_results,
            ))
        })
    }

    pub fn is_enabled(
        &self,
        toggle_name: &str,
        context: &Context,
        custom_strategy_results: Option<&HashMap<String, bool>>,
    ) -> Result<(Option<bool>, bool), EngineError> {
        guard(|| {
            let engine = self.lock();
            let enabled = evaluate(&engine, toggle_name, context, custom_strategy_results);
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
    /// unknown toggle
    pub fn check_variant(
        &self,
        toggle_name: &str,
        context: &Context,
        custom_strategy_results: Option<&HashMap<String, bool>>,
    ) -> Result<(Option<ExtendedVariantDef>, bool), EngineError> {
        guard(|| {
            Ok(resolve_variant(
                &self.lock(),
                toggle_name,
                context,
                custom_strategy_results,
            ))
        })
    }

    /// Like `check_variant`, and also records the toggle and the variant in
    /// the metrics; an unknown toggle is recorded as the disabled variant
    pub fn get_variant(
        &self,
        toggle_name: &str,
        context: &Context,
        custom_strategy_results: Option<&HashMap<String, bool>>,
    ) -> Result<(Option<ExtendedVariantDef>, bool), EngineError> {
        guard(|| {
            let engine = self.lock();
            let (variant, impression) =
                resolve_variant(&engine, toggle_name, context, custom_strategy_results);
            // An unknown toggle is counted as disabled
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
            Ok((variant, impression))
        })
    }

    pub fn count_toggle(&self, toggle_name: &str, enabled: bool) -> Result<(), EngineError> {
        guard(|| {
            self.lock().count_toggle(toggle_name, enabled);
            Ok(())
        })
    }

    pub fn get_metrics(&self) -> Result<Option<MetricBucket>, EngineError> {
        guard(|| Ok(self.lock().get_metrics(Utc::now())))
    }

    pub fn get_state(&self) -> Result<ClientFeatures, EngineError> {
        guard(|| Ok(self.lock().get_state()))
    }

    pub fn list_known_toggles(&self) -> Result<Vec<ToggleDefinition>, EngineError> {
        guard(|| Ok(self.lock().list_known_toggles()))
    }

    pub fn define_counter(&self, name: &str, help: &str) -> Result<(), EngineError> {
        guard(|| {
            self.lock().define_counter(MetricOptions::new(name, help));
            Ok(())
        })
    }

    pub fn inc_counter(
        &self,
        name: &str,
        value: i64,
        labels: Option<&MetricLabels>,
    ) -> Result<(), EngineError> {
        guard(|| {
            let engine = self.lock();
            match labels {
                Some(labels) if !labels.is_empty() => {
                    engine.inc_counter_with_labels(name, value, labels)
                }
                _ => engine.inc_counter_by(name, value),
            }
            Ok(())
        })
    }

    pub fn define_gauge(&self, name: &str, help: &str) -> Result<(), EngineError> {
        guard(|| {
            self.lock().define_gauge(MetricOptions::new(name, help));
            Ok(())
        })
    }
    pub fn set_gauge(
        &self,
        name: &str,
        value: f64,
        labels: Option<&MetricLabels>,
    ) -> Result<(), EngineError> {
        guard(|| {
            let engine = self.lock();
            match labels {
                Some(labels) if !labels.is_empty() => {
                    engine.set_gauge_with_labels(name, value, labels)
                }
                _ => engine.set_gauge(name, value),
            }
            Ok(())
        })
    }

    pub fn define_histogram(
        &self,
        name: &str,
        help: &str,
        buckets: Vec<f64>,
    ) -> Result<(), EngineError> {
        guard(|| {
            self.lock()
                .define_histogram(BucketMetricOptions::new(name, help, buckets));
            Ok(())
        })
    }

    pub fn observe_histogram(
        &self,
        name: &str,
        value: f64,
        labels: Option<&MetricLabels>,
    ) -> Result<(), EngineError> {
        guard(|| {
            let engine = self.lock();
            match labels {
                Some(labels) if !labels.is_empty() => {
                    engine.observe_histogram_with_labels(name, value, labels)
                }
                _ => engine.observe_histogram(name, value),
            }
            Ok(())
        })
    }

    pub fn collect_impact_metrics(&self) -> Result<Vec<CollectedMetric>, EngineError> {
        guard(|| Ok(self.lock().collect_impact_metrics()))
    }

    pub fn restore_impact_metrics(&self, json: &str) -> Result<(), EngineError> {
        guard(|| {
            let metrics: Vec<CollectedMetric> =
                serde_json::from_str(json).map_err(|e| EngineError::InvalidJson(e.to_string()))?;
            self.lock().restore_impact_metrics(metrics);
            Ok(())
        })
    }
}

/// The variant and whether the toggle is enabled (two evaluations, like the
/// shim's `check_variant`), and the impression flag
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

fn evaluate(
    engine: &EngineState,
    toggle_name: &str,
    context: &Context,
    custom_strategy_results: Option<&HashMap<String, bool>>,
) -> Option<bool> {
    engine.check_enabled(&EnrichedContext::from(
        context,
        toggle_name,
        custom_strategy_results,
    ))
}

pub fn guard<T, E: From<EngineError>>(action: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    panic::catch_unwind(AssertUnwindSafe(action)).unwrap_or_else(|_| Err(EngineError::Panic.into()))
}

#[cfg(test)]
mod tests {
    use std::panic::{self, AssertUnwindSafe};

    use unleash_types::client_features::{ClientFeature, ClientFeatures, Strategy};
    use unleash_yggdrasil::Context;

    use super::Engine;

    #[test]
    fn when_requesting_a_toggle_that_does_not_exist_then_a_response_with_no_error_and_not_found_is_returned(
    ) {
        let engine = Engine::default();
        let context: Context = serde_json::from_str("{}").unwrap();

        let enabled = engine
            .check_enabled("some-toggle", &context, None)
            .expect("Expected no error");

        assert!(enabled.is_none());
    }

    #[test]
    fn when_requesting_a_toggle_that_does_exist_and_is_enabled_then_a_response_with_no_error_and_enabled_status_is_returned(
    ) {
        let engine = Engine::default();
        let toggle_under_test = "some-toggle";
        let context: Context = serde_json::from_str("{}").unwrap();

        let client_features = ClientFeatures {
            features: vec![ClientFeature {
                name: toggle_under_test.into(),
                enabled: true,
                strategies: Some(vec![Strategy {
                    name: "default".into(),
                    constraints: None,
                    parameters: None,
                    segments: None,
                    sort_order: None,
                    variants: None,
                }]),
                ..Default::default()
            }],
            query: None,
            segments: None,
            version: 2,
            meta: None,
        };

        let warnings = engine
            .take_state(&serde_json::to_string(&client_features).unwrap())
            .expect("Expected no error");
        let enabled = engine
            .check_enabled(toggle_under_test, &context, None)
            .expect("Expected no error");

        assert_eq!(enabled, Some(true));
        assert!(warnings.is_none());
    }

    #[test]
    fn it_taketh_the_state() {
        let engine = Engine::default();
        let toggle_under_test = "some-toggle";
        let appname_test = "the-app";

        let client_features = ClientFeatures {
            features: vec![ClientFeature {
                name: toggle_under_test.into(),
                enabled: true,
                impression_data: Some(true),
                strategies: Some(vec![Strategy {
                    name: "default".into(),
                    constraints: None,
                    parameters: None,
                    segments: None,
                    sort_order: None,
                    variants: None,
                }]),
                ..Default::default()
            }],
            query: None,
            segments: None,
            version: 2,
            meta: None,
        };
        let serialised = serde_json::to_string(&client_features).unwrap();
        engine.take_state(&serialised).unwrap();

        let context: Context =
            serde_json::from_value(serde_json::json!({ "appName": appname_test })).unwrap();
        let (enabled, _) = engine
            .is_enabled(toggle_under_test, &context, None)
            .unwrap();

        assert_eq!(enabled, Some(true));
    }

    #[test]
    fn is_enabled_counts_known_toggles_and_leaves_unknown_ones_to_the_caller() {
        let engine = Engine::default();
        let client_features = ClientFeatures {
            features: vec![ClientFeature {
                name: "some-toggle".into(),
                enabled: true,
                strategies: Some(vec![Strategy {
                    name: "default".into(),
                    constraints: None,
                    parameters: None,
                    segments: None,
                    sort_order: None,
                    variants: None,
                }]),
                ..Default::default()
            }],
            query: None,
            segments: None,
            version: 2,
            meta: None,
        };
        engine
            .take_state(&serde_json::to_string(&client_features).unwrap())
            .unwrap();
        let context = Context::default();

        engine.is_enabled("some-toggle", &context, None).unwrap();
        engine.check_enabled("some-toggle", &context, None).unwrap();
        engine.is_enabled("unknown-toggle", &context, None).unwrap();
        engine.count_toggle("unknown-toggle", true).unwrap();

        let toggles = engine
            .get_metrics()
            .unwrap()
            .expect("Expected metrics")
            .toggles;

        assert_eq!(
            (toggles["some-toggle"].yes, toggles["some-toggle"].no),
            (1, 0)
        );
        assert_eq!(
            (toggles["unknown-toggle"].yes, toggles["unknown-toggle"].no),
            (1, 0)
        );
    }

    #[test]
    fn get_metrics_is_none_when_nothing_was_counted_and_resets_the_counts() {
        let engine = Engine::default();

        assert!(engine.get_metrics().unwrap().is_none());

        engine.count_toggle("some-toggle", false).unwrap();
        let toggles = engine
            .get_metrics()
            .unwrap()
            .expect("Expected metrics")
            .toggles;

        assert_eq!(
            (toggles["some-toggle"].yes, toggles["some-toggle"].no),
            (0, 1)
        );
        assert!(engine.get_metrics().unwrap().is_none());
    }

    #[test]
    fn get_variant_resolves_and_counts_known_and_unknown_toggles() {
        let engine = Engine::default();
        let state = serde_json::json!({
            "version": 2,
            "features": [
                {
                    "name": "with-variant",
                    "enabled": true,
                    "impressionData": true,
                    "strategies": [{"name": "default"}],
                    "variants": [{
                        "name": "blue",
                        "weight": 1000,
                        "payload": {"type": "string", "value": "b"}
                    }]
                },
                {
                    "name": "disabled-toggle",
                    "enabled": false,
                    "strategies": [{"name": "default"}]
                }
            ]
        });
        engine.take_state(&state.to_string()).unwrap();
        let context = Context::default();

        let (variant, impression) = engine.get_variant("with-variant", &context, None).unwrap();
        let variant = variant.expect("Expected a variant");
        assert_eq!(
            (
                variant.name.as_str(),
                variant.enabled,
                variant.feature_enabled
            ),
            ("blue", true, true)
        );
        assert_eq!(variant.payload.expect("Expected a payload").value, "b");
        assert!(impression);

        let (variant, impression) = engine
            .get_variant("disabled-toggle", &context, None)
            .unwrap();
        let variant = variant.expect("Expected a variant");
        assert_eq!(
            (
                variant.name.as_str(),
                variant.enabled,
                variant.feature_enabled
            ),
            ("disabled", false, false)
        );
        assert!(!impression);

        let (variant, _) = engine.get_variant("unknown", &context, None).unwrap();
        assert!(variant.is_none());

        let toggles = engine
            .get_metrics()
            .unwrap()
            .expect("Expected metrics")
            .toggles;
        let counts = |name: &str, variant: &str| {
            let stats = &toggles[name];
            (stats.yes, stats.no, stats.variants[variant])
        };
        assert_eq!(counts("with-variant", "blue"), (1, 0, 1));
        assert_eq!(counts("disabled-toggle", "disabled"), (0, 1, 1));
        assert_eq!(counts("unknown", "disabled"), (0, 1, 1));
    }

    #[test]
    fn get_state_is_empty_before_take_state_and_the_taken_state_after() {
        let engine = Engine::default();

        assert!(engine.get_state().unwrap().features.is_empty());

        let state = serde_json::json!({
            "version": 1,
            "features": [{"name": "some-toggle", "enabled": true, "strategies": [{"name": "default"}]}]
        });
        engine.take_state(&state.to_string()).unwrap();

        let features = engine.get_state().unwrap().features;
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].name, "some-toggle");
    }

    #[test]
    fn list_known_toggles_is_empty_before_take_state_and_lists_the_toggles_after() {
        let engine = Engine::default();

        assert!(engine.list_known_toggles().unwrap().is_empty());

        let state = serde_json::json!({
            "version": 1,
            "features": [{
                "name": "some-toggle",
                "type": "release",
                "project": "some-project",
                "enabled": true,
                "strategies": [{"name": "default"}]
            }]
        });
        engine.take_state(&state.to_string()).unwrap();

        let toggles = engine.list_known_toggles().unwrap();
        assert_eq!(toggles.len(), 1);
        assert_eq!(
            (
                toggles[0].name.as_str(),
                toggles[0].project.as_str(),
                toggles[0].feature_type.as_deref()
            ),
            ("some-toggle", "some-project", Some("release"))
        );
    }

    #[test]
    fn check_variant_resolves_like_get_variant_without_counting() {
        let engine = Engine::default();
        let state = serde_json::json!({
            "version": 1,
            "features": [{
                "name": "with-variant",
                "enabled": true,
                "strategies": [{"name": "default"}],
                "variants": [{"name": "blue", "weight": 1000}]
            }]
        });
        engine.take_state(&state.to_string()).unwrap();
        let context = Context::default();

        let checked = engine
            .check_variant("with-variant", &context, None)
            .unwrap();
        assert!(engine.get_metrics().unwrap().is_none());

        let got = engine.get_variant("with-variant", &context, None).unwrap();
        assert_eq!(checked, got);

        let (unknown, _) = engine.check_variant("unknown", &context, None).unwrap();
        assert!(unknown.is_none());
        let toggles = engine.get_metrics().unwrap().unwrap().toggles;
        assert!(!toggles.contains_key("unknown"));
    }

    #[test]
    fn when_a_thread_panicked_while_holding_the_lock_then_the_engine_keeps_working() {
        let engine = Engine::default();
        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
            let _held = engine.state.lock().unwrap();
            panic!("poisoning the lock on purpose");
        }));
        assert!(engine.state.is_poisoned());

        let result = engine.check_enabled("some-toggle", &Context::default(), None);

        assert!(result.is_ok());
    }
}
