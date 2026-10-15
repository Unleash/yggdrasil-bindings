#![deny(clippy::all)]

use std::{borrow::Cow, collections::HashMap, fmt};

use serde::{
    de::{
        self,
        value::{MapAccessDeserializer, SeqAccessDeserializer},
        MapAccess, SeqAccess, Visitor,
    },
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use unleash_yggdrasil::{
    state::{
        EnrichedContext, ExternalResultsCow, ExternalResultsRef, PropertiesCow, PropertiesRef,
    },
    EngineState, UpdateMessage,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct NodeEngine {
    inner: Option<EngineState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnabledResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    pub impression_data: bool,
}

struct BorrowedString<'a>(Cow<'a, str>);

impl<'de> Deserialize<'de> for BorrowedString<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = BorrowedString<'de>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string")
            }

            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BorrowedString(Cow::Borrowed(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BorrowedString(Cow::Owned(value.to_owned())))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BorrowedString(Cow::Owned(value)))
            }
        }

        deserializer.deserialize_str(StringVisitor)
    }
}

struct ContextValue<'a>(Option<Cow<'a, str>>);

impl<'de> Deserialize<'de> for ContextValue<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = ContextValue<'de>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON value")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ContextValue(None))
            }

            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ContextValue(Some(Cow::Borrowed(value))))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ContextValue(Some(Cow::Owned(value.to_owned()))))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ContextValue(Some(Cow::Owned(value))))
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ContextValue(Some(Cow::Borrowed(if value {
                    "true"
                } else {
                    "false"
                }))))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ContextValue(Some(Cow::Owned(value.to_string()))))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ContextValue(Some(Cow::Owned(value.to_string()))))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let number = serde_json::Number::from_f64(value)
                    .ok_or_else(|| E::custom("non-finite context number"))?;

                Ok(ContextValue(Some(Cow::Owned(number.to_string()))))
            }

            fn visit_seq<A>(self, sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let value = Value::deserialize(SeqAccessDeserializer::new(sequence))?;

                Ok(ContextValue(Some(Cow::Owned(value.to_string()))))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let value = Value::deserialize(MapAccessDeserializer::new(map))?;

                Ok(ContextValue(Some(Cow::Owned(value.to_string()))))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ExplicitProperties<'a>(Vec<(Cow<'a, str>, Option<Cow<'a, str>>)>);

impl<'de> Deserialize<'de> for ExplicitProperties<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PropertiesVisitor;

        impl<'de> Visitor<'de> for PropertiesVisitor {
            type Value = ExplicitProperties<'de>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a properties object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut properties = Vec::with_capacity(map.size_hint().unwrap_or(0));

                while let Some(BorrowedString(key)) = map.next_key()? {
                    let ContextValue(value) = map.next_value()?;
                    properties.push((key, value));
                }

                Ok(ExplicitProperties(properties))
            }
        }

        deserializer.deserialize_map(PropertiesVisitor)
    }
}

struct ExternalResults<'a>(ExternalResultsCow<'a>);

impl<'de> Deserialize<'de> for ExternalResults<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResultsVisitor;

        impl<'de> Visitor<'de> for ResultsVisitor {
            type Value = ExternalResults<'de>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object of boolean strategy results")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut results = HashMap::with_capacity(map.size_hint().unwrap_or(0));

                while let Some(BorrowedString(key)) = map.next_key()? {
                    results.insert(key, map.next_value::<bool>()?);
                }

                Ok(ExternalResults(results))
            }
        }

        deserializer.deserialize_map(ResultsVisitor)
    }
}

impl<'a> ParsedContext<'a> {
    fn enriched(&self) -> EnrichedContext<'_> {
        EnrichedContext {
            user_id: self.user_id.as_deref(),
            session_id: self.session_id.as_deref(),
            environment: self.environment.as_deref(),
            app_name: self.app_name.as_deref(),
            current_time: self.current_time.as_deref(),
            remote_address: self.remote_address.as_deref(),
            runtime_hostname: self.runtime_hostname.as_deref(),

            properties: Some(PropertiesRef::Cows(&self.properties)),
            external_results: self.external_results.as_ref().map(ExternalResultsRef::Cows),

            toggle_name: self.toggle_name.as_ref(),
        }
    }
}

#[derive(Default)]
struct ParsedContext<'a> {
    user_id: Option<Cow<'a, str>>,
    session_id: Option<Cow<'a, str>>,
    environment: Option<Cow<'a, str>>,
    app_name: Option<Cow<'a, str>>,
    current_time: Option<Cow<'a, str>>,
    remote_address: Option<Cow<'a, str>>,
    runtime_hostname: Option<Cow<'a, str>>,

    toggle_name: Cow<'a, str>,
    properties: PropertiesCow<'a>,
    external_results: Option<ExternalResultsCow<'a>>,
}

impl<'de> Deserialize<'de> for ParsedContext<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContextVisitor;

        impl<'de> Visitor<'de> for ContextVisitor {
            type Value = ParsedContext<'de>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an enriched context object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut context = ParsedContext::default();
                let mut explicit = None;
                let mut seen = 0_u16;

                while let Some(BorrowedString(key)) = map.next_key()? {
                    let field = match key.as_ref() {
                        "userId" => Some((0, "userId")),
                        "sessionId" => Some((1, "sessionId")),
                        "environment" => Some((2, "environment")),
                        "appName" => Some((3, "appName")),
                        "currentTime" => Some((4, "currentTime")),
                        "remoteAddress" => Some((5, "remoteAddress")),
                        "runtimeHostname" => Some((6, "runtimeHostname")),
                        "toggleName" => Some((7, "toggleName")),
                        "properties" => Some((8, "properties")),
                        "externalResults" => Some((9, "externalResults")),
                        _ => None,
                    };

                    if let Some((index, name)) = field {
                        let bit = 1_u16 << index;

                        if seen & bit != 0 {
                            return Err(de::Error::duplicate_field(name));
                        }
                        seen |= bit;

                        match index {
                            0 => context.user_id = map.next_value::<ContextValue>()?.0,
                            1 => context.session_id = map.next_value::<ContextValue>()?.0,
                            2 => context.environment = map.next_value::<ContextValue>()?.0,
                            3 => context.app_name = map.next_value::<ContextValue>()?.0,
                            4 => context.current_time = map.next_value::<ContextValue>()?.0,
                            5 => context.remote_address = map.next_value::<ContextValue>()?.0,
                            6 => context.runtime_hostname = map.next_value::<ContextValue>()?.0,
                            7 => {
                                context.toggle_name = map.next_value::<BorrowedString>()?.0;
                            }
                            8 => {
                                explicit = map.next_value::<Option<ExplicitProperties>>()?;
                            }
                            9 => {
                                context.external_results = map
                                    .next_value::<Option<ExternalResults>>()?
                                    .map(|results| results.0);
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        let ContextValue(value) = map.next_value()?;

                        if let Some(value) = value {
                            context.properties.insert(key, value);
                        } else {
                            context.properties.remove(key.as_ref());
                        }
                    }
                }

                if seen & (1 << 7) == 0 {
                    return Err(de::Error::missing_field("toggleName"));
                }

                if let Some(ExplicitProperties(explicit)) = explicit {
                    context.properties.reserve(explicit.len());

                    for (key, value) in explicit {
                        if let Some(value) = value {
                            context.properties.insert(key, value);
                        } else {
                            context.properties.remove(key.as_ref());
                        }
                    }
                }

                Ok(context)
            }
        }

        deserializer.deserialize_map(ContextVisitor)
    }
}

impl Default for NodeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl NodeEngine {
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
    pub fn take_state(&mut self, raw_state: &str) -> Result<JsValue, JsError> {
        let engine = self
            .inner
            .as_mut()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let client_features = serde_json::from_str::<UpdateMessage>(raw_state)
            .map_err(|err| JsError::new(&format!("Failed to parse client features: {err}")))?;

        let warnings = engine.take_state(client_features);

        Ok(match warnings.filter(|w| !w.is_empty()) {
            Some(warnings) => JsValue::from_str(
                &warnings
                    .iter()
                    .map(|w| format!("{}: {}", w.toggle_name, w.message))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            None => JsValue::NULL,
        })
    }

    #[wasm_bindgen(js_name = isEnabled)]
    pub fn is_enabled(&self, enriched_context: &str) -> Result<JsValue, JsError> {
        let engine = self
            .inner
            .as_ref()
            .ok_or_else(|| JsError::new("Engine state is not initialized"))?;

        let message: ParsedContext<'_> = serde_json::from_str(enriched_context)
            .map_err(|err| JsError::new(&format!("Failed to parse enriched context: {err}")))?;

        let context = message.enriched();

        serde_wasm_bindgen::to_value(&EnabledResult {
            enabled: engine.check_enabled(&context),
            impression_data: engine.should_emit_impression_event(context.toggle_name),
        })
        .map_err(|err| JsError::new(&err.to_string()))
    }
}
