

use std::collections::HashMap;

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

#[derive(Default)]
pub struct Engine {
    state: EngineState,
}

impl Engine {
    pub fn take_state(&self, json: &str) -> Result<Option<Vec<EvalWarning>>, serde_json::Error> {
        let message: UpdateMessage = serde_json::from_str(json)?;
        Ok(self.state.take_state(message))
    }

    pub fn is_enabled(
        &self,
        toggle_name: &str,
        context: &Context,
        custom_strategy_results: Option<&HashMap<String, bool>>,
    ) -> (Option<bool>, bool) {
        let engine = &self.state;
        let enabled = evaluate(engine, toggle_name, context, custom_strategy_results);
        let impression = engine.should_emit_impression_event(toggle_name);
        if let Some(enabled) = enabled {
            engine.count_toggle(toggle_name, enabled);
        }
        (enabled, impression)
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
    ) -> (Option<ExtendedVariantDef>, bool) {
        resolve_variant(&self.state, toggle_name, context, custom_strategy_results)
    }

    /// Like `check_variant`, and also records the toggle and the variant in
    /// the metrics; an unknown toggle is recorded as the disabled variant
    pub fn get_variant(
        &self,
        toggle_name: &str,
        context: &Context,
        custom_strategy_results: Option<&HashMap<String, bool>>,
    ) -> (Option<ExtendedVariantDef>, bool) {
        let engine = &self.state;
        let (variant, impression) =
            resolve_variant(engine, toggle_name, context, custom_strategy_results);
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
        (variant, impression)
    }

    pub fn count_toggle(&self, toggle_name: &str, enabled: bool) {
        self.state.count_toggle(toggle_name, enabled);
    }

    pub fn get_metrics(&self) -> Option<MetricBucket> {
        self.state.get_metrics(Utc::now())
    }

    pub fn get_state(&self) -> ClientFeatures {
        self.state.get_state()
    }

    pub fn list_known_toggles(&self) -> Vec<ToggleDefinition> {
        self.state.list_known_toggles()
    }

    pub fn define_counter(&self, name: &str, help: &str) {
        self.state.define_counter(MetricOptions::new(name, help));
    }

    pub fn inc_counter(&self, name: &str, value: i64, labels: Option<&MetricLabels>) {
        let engine = &self.state;
        match labels {
            Some(labels) if !labels.is_empty() => {
                engine.inc_counter_with_labels(name, value, labels)
            }
            _ => engine.inc_counter_by(name, value),
        }
    }

    pub fn define_gauge(&self, name: &str, help: &str) {
        self.state.define_gauge(MetricOptions::new(name, help));
    }

    pub fn set_gauge(&self, name: &str, value: f64, labels: Option<&MetricLabels>) {
        let engine = &self.state;
        match labels {
            Some(labels) if !labels.is_empty() => engine.set_gauge_with_labels(name, value, labels),
            _ => engine.set_gauge(name, value),
        }
    }

    pub fn define_histogram(&self, name: &str, help: &str, buckets: Vec<f64>) {
        self.state
            .define_histogram(BucketMetricOptions::new(name, help, buckets));
    }

    pub fn observe_histogram(&self, name: &str, value: f64, labels: Option<&MetricLabels>) {
        let engine = &self.state;
        match labels {
            Some(labels) if !labels.is_empty() => {
                engine.observe_histogram_with_labels(name, value, labels)
            }
            _ => engine.observe_histogram(name, value),
        }
    }

    pub fn collect_impact_metrics(&self) -> Vec<CollectedMetric> {
        self.state.collect_impact_metrics()
    }

    pub fn restore_impact_metrics(&self, json: &str) -> Result<(), serde_json::Error> {
        let metrics: Vec<CollectedMetric> = serde_json::from_str(json)?;
        self.state.restore_impact_metrics(metrics);
        Ok(())
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

#[cfg(test)]
mod tests {
    use unleash_types::client_features::{ClientFeature, ClientFeatures, Strategy};
    use unleash_yggdrasil::Context;

    use super::Engine;

    #[test]
    fn when_requesting_a_toggle_that_does_not_exist_then_a_response_with_no_error_and_not_found_is_returned(
    ) {
        let engine = Engine::default();
        let context: Context = serde_json::from_str("{}").unwrap();

        let (enabled, _) = engine.is_enabled("some-toggle", &context, None);

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
        let (enabled, _) = engine.is_enabled(toggle_under_test, &context, None);

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
        let (enabled, _) = engine.is_enabled(toggle_under_test, &context, None);

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

        engine.is_enabled("some-toggle", &context, None);
        engine.is_enabled("unknown-toggle", &context, None);
        engine.count_toggle("unknown-toggle", true);

        let toggles = engine.get_metrics().expect("Expected metrics").toggles;

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

        assert!(engine.get_metrics().is_none());

        engine.count_toggle("some-toggle", false);
        let toggles = engine.get_metrics().expect("Expected metrics").toggles;

        assert_eq!(
            (toggles["some-toggle"].yes, toggles["some-toggle"].no),
            (0, 1)
        );
        assert!(engine.get_metrics().is_none());
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

        let (variant, impression) = engine.get_variant("with-variant", &context, None);
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

        let (variant, impression) = engine.get_variant("disabled-toggle", &context, None);
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

        let (variant, _) = engine.get_variant("unknown", &context, None);
        assert!(variant.is_none());

        let toggles = engine.get_metrics().expect("Expected metrics").toggles;
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

        assert!(engine.get_state().features.is_empty());

        let state = serde_json::json!({
            "version": 1,
            "features": [{"name": "some-toggle", "enabled": true, "strategies": [{"name": "default"}]}]
        });
        engine.take_state(&state.to_string()).unwrap();

        let features = engine.get_state().features;
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].name, "some-toggle");
    }

    #[test]
    fn list_known_toggles_is_empty_before_take_state_and_lists_the_toggles_after() {
        let engine = Engine::default();

        assert!(engine.list_known_toggles().is_empty());

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

        let toggles = engine.list_known_toggles();
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
}
