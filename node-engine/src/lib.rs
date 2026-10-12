#![deny(clippy::all)]

use napi_derive::napi;
use serde_json;
use unleash_yggdrasil::{state::EnrichedContext, EngineState, UpdateMessage};

#[napi]
pub struct NodeEngine {
    inner: Option<EngineState>,
}

#[napi]
pub struct EnabledResult {
    pub enabled: Option<bool>,
    pub impression_data: bool,
}

#[napi]
impl NodeEngine {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Some(EngineState::default()),
        }
    }

    #[napi]
    pub fn destroy(&mut self) {
        self.inner = None;
    }

    #[napi]
    pub fn take_state(&mut self, raw_state: String) -> napi::Result<Option<String>> {
        let engine = self
            .inner
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("Engine state is not initialized"))?;

        let client_features = serde_json::from_str::<UpdateMessage>(&raw_state).map_err(|err| {
            napi::Error::from_reason(format!("Failed to parse client features: {}", err))
        })?;

        let warnings = engine.take_state(client_features);

        Ok(warnings.filter(|w| !w.is_empty()).map(|warnings| {
            warnings
                .iter()
                .map(|w| format!("{}: {}", w.toggle_name, w.message))
                .collect::<Vec<_>>()
                .join("\n")
        }))
    }

    #[napi]
    pub fn is_enabled(&self, feature_name: String) -> napi::Result<EnabledResult> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| napi::Error::from_reason("Engine state is not initialized"))?;

        let context = unleash_yggdrasil::Context::default();

        let enriched_context = EnrichedContext::from(&context, &feature_name, None);

        Ok(EnabledResult {
            enabled: engine.check_enabled(&enriched_context),
            impression_data: engine.should_emit_impression_event(&feature_name),
        })
    }
}
