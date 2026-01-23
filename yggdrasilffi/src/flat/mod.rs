#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

use flatbuffers::root;
use std::borrow::Cow;
use std::ffi::{c_char, c_void};

use crate::flat::serialisation::{Buf, TakeStateResult};
use crate::{get_json, ManagedEngine, RawPointerDataType};
use chrono::Utc;
use messaging::messaging::{
    BuiltInStrategies, ContextMessage, FeatureDefs, MetricsResponse, Response, TakeStateResponse,
    Variant,
};
use serialisation::{FlatError, FlatMessage, ResponseMessage};
use std::collections::{BTreeMap, HashMap};
use std::mem::forget;
use std::panic;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard};
use unleash_types::client_metrics::MetricBucket;
use unleash_yggdrasil::state::{
    EnrichedContext, ExternalResultsCow, ExternalResultsRef, PropertiesCow, PropertiesRef,
};
use unleash_yggdrasil::{ExtendedVariantDef, ToggleDefinition, UpdateMessage, KNOWN_STRATEGIES};

mod jni_bridge;
mod serialisation;
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]
mod messaging {
    #![allow(dead_code)]
    #![allow(non_snake_case)]
    #![allow(warnings)]
    include!("enabled-message_generated.rs");
}

///
/// # Safety
///
/// Must be freed on the calling side, we forget this in order to be able to pass it back to the caller
unsafe fn get_engine(engine_ptr: *mut c_void) -> Result<ManagedEngine, FlatError> {
    if engine_ptr.is_null() {
        return Err(FlatError::NullError);
    }
    let arc_instance = Arc::from_raw(engine_ptr as *const RawPointerDataType);

    let cloned_arc = arc_instance.clone();
    forget(arc_instance);

    Ok(cloned_arc)
}

fn recover_lock<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[no_mangle]
pub extern "C" fn flat_buf_free(buf: Buf) {
    if buf.ptr.is_null() {
        return;
    }
    unsafe { drop(Vec::from_raw_parts(buf.ptr, buf.len, buf.cap)) }
}

#[no_mangle]
/// # Safety
///
/// Should only be called from the thread that created the engine, to ensure the engine pointer is valid
/// The return value should also be freed using flat_buf_free
pub unsafe fn flat_take_state(engine_pointer: *mut c_void, toggles_pointer: *const c_char) -> Buf {
    let result = guard_result::<TakeStateResult, _>(|| {
        let guard = get_engine(engine_pointer)?;
        let mut engine = recover_lock(&guard);
        let toggles: UpdateMessage = get_json(toggles_pointer)
            .map_err(|_| FlatError::InvalidState("Your features does not parse".to_string()))?;
        let res = engine.take_state(toggles);
        let feature_strategies_map = engine
            .get_state()
            .features
            .iter()
            .map(|feature| {
                let name = feature.name.clone();
                let strategies = feature
                    .strategies
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|strategy| {
                        let params: BTreeMap<String, String> = strategy
                            .parameters
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .collect();
                        (strategy.name.clone(), params)
                    })
                    .collect::<BTreeMap<_, _>>();
                (name, strategies)
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(warnings) = res {
            Ok(Some(TakeStateResult {
                warnings,
                error: None,
                feature_strategies_map,
            }))
        } else {
            Ok(Some(TakeStateResult {
                warnings: vec![],
                error: None,
                feature_strategies_map,
            }))
        }
    });
    TakeStateResponse::build_response(result)
}

fn extract_context_properties<'a>(buffer: &ContextMessage<'a>) -> Option<PropertiesCow<'a>> {
    let props_buffer = buffer.properties()?;
    let mut properties = PropertiesCow::with_capacity(props_buffer.len());

    for p in props_buffer.iter() {
        if let Some(v) = p.value() {
            properties.insert(Cow::Borrowed(p.key()), Cow::Borrowed(v));
        }
    }

    Some(properties)
}

fn extract_external_results<'a>(buffer: &ContextMessage<'a>) -> Option<ExternalResultsCow<'a>> {
    let entries = buffer.custom_strategies_results()?;
    let mut external_results = HashMap::with_capacity(entries.len());

    for entry in entries.iter() {
        external_results.insert(Cow::Borrowed(entry.key()), entry.value());
    }

    Some(external_results)
}

#[no_mangle]
/// # Safety
///
/// passing an invalid engine_ptr, message_ptr or message_len will cause UB
/// the returned Buf should be freed by calling flat_buf_free, otherwise you're leaking memory
pub unsafe extern "C" fn flat_check_enabled(
    engine_ptr: *mut c_void,
    message_ptr: u64,
    message_len: u64,
) -> Buf {
    let enabled = guard_result::<ResponseMessage<bool>, _>(|| {
        let bytes =
            unsafe { std::slice::from_raw_parts(message_ptr as *const u8, message_len as usize) };
        let ctx =
            root::<ContextMessage>(bytes).map_err(|e| FlatError::InvalidContext(e.to_string()))?;

        let properties = extract_context_properties(&ctx);
        let external_results = extract_external_results(&ctx);

        let context = EnrichedContext {
            toggle_name: ctx.toggle_name().ok_or(FlatError::MissingFlagName)?,
            user_id: ctx.user_id(),
            session_id: ctx.session_id(),
            environment: ctx.environment(),
            app_name: ctx.app_name(),
            current_time: ctx.current_time(),
            remote_address: ctx.remote_address(),
            properties: properties.as_ref().map(PropertiesRef::Cows),
            external_results: external_results.as_ref().map(ExternalResultsRef::Cows),
            runtime_hostname: ctx.runtime_hostname(),
        };

        let lock = get_engine(engine_ptr)?;
        let engine = recover_lock(&lock);

        let enabled = engine.check_enabled(&context);
        let impression_data = engine.should_emit_impression_event(&context.toggle_name);
        engine.count_toggle(&context.toggle_name, enabled.unwrap_or(false));

        Ok(Some(ResponseMessage {
            message: enabled,
            impression_data,
        }))
    });

    Response::build_response(enabled)
}
/// Evaluates the variant for the requested feature toggle contained in the ContextMessage
///
/// # Safety
///
/// passing an invalid engine_ptr, message_ptr or message_len will cause UB
/// the returned Buf should be freed by calling flat_buf_free, otherwise you're leaking memory
#[no_mangle]
pub unsafe extern "C" fn flat_check_variant(
    engine_ptr: *mut c_void,
    message_ptr: u64,
    message_len: u64,
) -> Buf {
    let variant = guard_result::<ResponseMessage<ExtendedVariantDef>, _>(|| {
        let bytes =
            unsafe { std::slice::from_raw_parts(message_ptr as *const u8, message_len as usize) };
        let ctx =
            root::<ContextMessage>(bytes).map_err(|e| FlatError::InvalidContext(e.to_string()))?;

        let properties = extract_context_properties(&ctx);
        let external_results = extract_external_results(&ctx);

        let context = EnrichedContext {
            toggle_name: ctx.toggle_name().ok_or(FlatError::MissingFlagName)?,
            user_id: ctx.user_id(),
            session_id: ctx.session_id(),
            environment: ctx.environment(),
            app_name: ctx.app_name(),
            current_time: ctx.current_time(),
            remote_address: ctx.remote_address(),
            properties: properties.as_ref().map(PropertiesRef::Cows),
            external_results: external_results.as_ref().map(ExternalResultsRef::Cows),
            runtime_hostname: ctx.runtime_hostname(),
        };

        let lock = get_engine(engine_ptr)?;
        let engine = recover_lock(&lock);
        let base_variant = engine.check_variant(&context);
        let toggle_enabled = engine.check_enabled(&context).unwrap_or_default();
        let impression_data = engine.should_emit_impression_event(&context.toggle_name);
        engine.count_toggle(&context.toggle_name, toggle_enabled);
        if let Some(v) = base_variant.clone() {
            engine.count_variant(&context.toggle_name, &v.name);
        }
        let message = base_variant.map(|variant| variant.to_enriched_response(toggle_enabled));
        Ok(Some(ResponseMessage {
            message,
            impression_data,
        }))
    });
    Variant::build_response(variant)
}

/// Get the list of toggles the engine knows about.
///
/// # Safety
///
/// passing an invalid engine_ptr will cause UB
/// the returned Buf should be freed by calling flat_buf_free, otherwise you're leaking memory
#[no_mangle]
pub unsafe extern "C" fn flat_list_known_toggles(engine_ptr: *mut c_void) -> Buf {
    let toggles = guard_result::<Vec<ToggleDefinition>, _>(|| {
        let guard = get_engine(engine_ptr)?;
        let engine = recover_lock(&guard);
        Ok(Some(engine.list_known_toggles()))
    });
    if let Ok(toggles) = toggles {
        FeatureDefs::build_response(toggles.unwrap_or_default())
    } else {
        FeatureDefs::build_response(vec![])
    }
}

///
/// Returns a list of built-in strategies
///
/// # Safety
///
/// the returned Buf should be freed by calling flat_buf_free, otherwise you're leaking memory
#[no_mangle]
pub unsafe extern "C" fn flat_built_in_strategies() -> Buf {
    BuiltInStrategies::build_response(KNOWN_STRATEGIES)
}

/// Get the accumulated metrics for the current bucket
///
/// # Safety
///
/// passing an invalid engine_ptr will cause UB
/// the returned Buf should be freed by calling flat_buf_free, otherwise you're leaking memory
///
#[no_mangle]
pub unsafe extern "C" fn flat_get_metrics(engine_pointer: *mut c_void) -> Buf {
    let result = guard_result::<MetricBucket, _>(|| {
        let guard = get_engine(engine_pointer)?;
        let mut engine = recover_lock(&guard);
        Ok(engine.get_metrics(Utc::now()))
    });
    MetricsResponse::build_response(result)
}

fn guard_result<T, F>(action: F) -> Result<Option<T>, FlatError>
where
    F: FnOnce() -> Result<Option<T>, FlatError>,
{
    panic::catch_unwind(AssertUnwindSafe(action)).unwrap_or_else(|_| Err(FlatError::Panic))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flat::messaging::messaging::ContextMessageBuilder;
    use crate::{free_engine, free_response, get_state, new_engine, take_state};
    use flatbuffers::{FlatBufferBuilder, WIPOffset};
    use serde_json::Value;
    use std::ffi::CString;
    use unleash_types::client_features::{ClientFeature, ClientFeatures, Strategy};

    pub fn allocate(size: usize) -> *mut u8 {
        let mut buf = Vec::with_capacity(size);
        let ptr = buf.as_mut_ptr();
        forget(buf);
        ptr
    }

    #[test]
    fn it_taketh_the_state() {
        let engine_ptr = new_engine();
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
        let c_serialised = CString::new(serialised).unwrap();
        let json_ptr = c_serialised.as_ptr();

        unsafe {
            let response_ptr = take_state(engine_ptr, json_ptr) as *mut i8;
            free_response(response_ptr);
            let mut builder = FlatBufferBuilder::with_capacity(128);
            let toggle_name: WIPOffset<&str> = builder.create_string(&toggle_under_test).into();
            let app_name: WIPOffset<&str> = builder.create_string(&appname_test).into();

            let mut context_builder = ContextMessageBuilder::new(&mut builder);
            context_builder.add_toggle_name(toggle_name);
            context_builder.add_app_name(app_name);
            let offset = context_builder.finish();
            builder.finish(offset, None);
            let finished_data = builder.finished_data().to_vec();

            let context_len = finished_data.len();
            let context_ptr = allocate(context_len);

            std::ptr::copy_nonoverlapping(finished_data.as_ptr(), context_ptr, context_len);

            let flat_response_buf: Buf =
                flat_check_enabled(engine_ptr, context_ptr as u64, context_len as u64);
            let slice: &[u8] =
                std::slice::from_raw_parts(flat_response_buf.ptr, flat_response_buf.len as usize);
            let resp: Response = root::<Response>(slice).unwrap();
            assert!(resp.enabled());
            flat_buf_free(flat_response_buf);
        }
    }

    #[test]
    fn take_state_updates_get_state() {
        let engine_ptr = new_engine();
        let toggle_under_test = "some-toggle";
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
        let c_serialised = CString::new(serialised.clone()).unwrap();
        let json_ptr = c_serialised.as_ptr();
        unsafe {
            let _ = take_state(engine_ptr, json_ptr);
            let state = get_state(engine_ptr);
            let state_from_engine = CString::from_raw(state as *mut _);
            let state_as_json: Value =
                serde_json::from_str(state_from_engine.to_str().unwrap()).unwrap();
            let value = state_as_json.as_object().unwrap().get("value").unwrap();
            let features: ClientFeatures = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(features, client_features);
            free_engine(engine_ptr);
        }
    }

    #[test]
    fn flat_take_state_returns_all_features_and_their_strategies() {
        let engine_ptr = new_engine();
        let toggle_under_test = "some-toggle";
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
        let c_serialised = CString::new(serialised.clone()).unwrap();
        let json_ptr = c_serialised.as_ptr();
        unsafe {
            let buf = flat_take_state(engine_ptr, json_ptr);
            let bytes: &[u8] = std::slice::from_raw_parts(buf.ptr, buf.len);
            let take_state_response = root::<TakeStateResponse>(bytes).unwrap();
            assert!(take_state_response.features().is_some());
            assert_eq!(take_state_response.features().map(|f| f.len()), Some(1));
            let feature = take_state_response.features().unwrap().get(0);
            assert_eq!(feature.feature_name(), Some(toggle_under_test));
            assert!(feature.strategies().is_some());
            let strategy = feature.strategies().unwrap().get(0);
            assert_eq!(strategy.name(), Some("default"));
        }
    }

    #[test]
    pub fn flat_take_state_also_works_with_custom_strategies() {
        let engine_ptr = new_engine();
        let custom_strategy_tests = include_str!("../../testfiles/custom-strategy-tests.json");
        let c_string = CString::new(custom_strategy_tests).unwrap();
        let json_ptr = c_string.as_ptr();
        unsafe {
            let buf = flat_take_state(engine_ptr, json_ptr);
            let bytes: &[u8] = std::slice::from_raw_parts(buf.ptr, buf.len);
            let take_state_response = root::<TakeStateResponse>(bytes).unwrap();
            assert!(take_state_response.features().is_some());
            assert_eq!(take_state_response.features().map(|f| f.len()), Some(3));
            let feature_map: HashMap<String, Vec<String>> = take_state_response
                .features()
                .unwrap()
                .iter()
                .map(|e| {
                    let feature_name = e.feature_name().map(|s| s.to_string()).unwrap();
                    let strategies: Vec<String> = e
                        .strategies()
                        .unwrap()
                        .iter()
                        .map(|s| s)
                        .map(|s| s.name().unwrap().to_string())
                        .collect();
                    (feature_name, strategies)
                })
                .collect();
            let custom_strategies = feature_map
                .get("Feature.Custom.Strategies")
                .unwrap()
                .clone();
            assert_eq!(custom_strategies.len(), 2);
            assert!(custom_strategies.contains(&"custom".to_string()));
            assert!(custom_strategies.contains(&"cus-tom".to_string()));
        }
    }

    #[test]
    pub fn flat_take_state_does_the_expected_thing() {
        let engine_ptr = new_engine();
        let custom_strategy_tests = include_str!("../../testfiles/custom_strategy_feature_d.json");
        let c_string = CString::new(custom_strategy_tests).unwrap();
        let json_ptr = c_string.as_ptr();
        unsafe {
            let buf = flat_take_state(engine_ptr, json_ptr);
            let bytes: &[u8] = std::slice::from_raw_parts(buf.ptr, buf.len);
            let take_state_response = root::<TakeStateResponse>(bytes).unwrap();
            assert!(take_state_response.features().is_some());
            assert_eq!(take_state_response.features().map(|f| f.len()), Some(1));
            let feature_map: HashMap<String, Vec<String>> = take_state_response
                .features()
                .unwrap()
                .iter()
                .map(|e| {
                    let feature_name = e.feature_name().map(|s| s.to_string()).unwrap();
                    let strategies: Vec<String> = e
                        .strategies()
                        .unwrap()
                        .iter()
                        .map(|s| s)
                        .map(|s| s.name().unwrap().to_string())
                        .collect();
                    (feature_name, strategies)
                })
                .collect();
            let custom_strategies = feature_map.get("Feature.D").unwrap().clone();
            assert_eq!(custom_strategies.len(), 1);
            assert!(custom_strategies.contains(&"custom".to_string()));
        }
    }
}
