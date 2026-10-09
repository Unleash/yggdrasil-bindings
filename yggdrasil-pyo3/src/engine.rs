//! Engine logic copied from the yggdrasilffi shim
//!  engine state behind a lock, panic guard, and the calls `is_enabled` needs
//!
//! only Rust with no PyO3 types, so it can later move into a library shared
//! with the shim to remove duplication

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard};

use unleash_yggdrasil::state::EnrichedContext;
use unleash_yggdrasil::{Context, EngineState, EvalWarning, UpdateMessage};

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

    pub fn count_toggle(&self, toggle_name: &str, enabled: bool) -> Result<(), EngineError> {
        guard(|| {
            self.lock().count_toggle(toggle_name, enabled);
            Ok(())
        })
    }
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

    use chrono::Utc;
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
            .lock()
            .get_metrics(Utc::now())
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
