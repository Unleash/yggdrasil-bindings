use chrono::DateTime;
use core::str;
use flatbuffers::root;

use serialisation::{ResponseMessage, FlatError, FlatMessage};
use messaging::messaging::{
    BuiltInStrategies, ContextMessage, CoreVersion, FeatureDefs, MetricsResponse, Response, Variant,
};

use unleash_yggdrasil::{
    EngineState, ExtendedVariantDef, KNOWN_STRATEGIES, UpdateMessage, state::EnrichedContext,
};
use std::alloc::{alloc, dealloc};
use std::mem::forget;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{alloc::Layout, collections::HashMap, slice};

mod serialisation;

#[allow(clippy::all)]
mod messaging {
    #![allow(dead_code)]
    #![allow(non_snake_case)]
    #![allow(warnings)]
    include!("enabled-message_generated.rs");
}

type RawPointerDataType = Mutex<EngineState>;
type ManagedEngine = Arc<RawPointerDataType>;

impl TryFrom<ContextMessage<'_>> for EnrichedContext {
    type Error = FlatError;

    fn try_from(value: ContextMessage) -> Result<Self, Self::Error> {
        let toggle_name = value
            .toggle_name()
            .ok_or(FlatError::MissingFlagName)?;

        let context = EnrichedContext {
            toggle_name: toggle_name.to_string(),
            runtime_hostname: value.runtime_hostname().map(|f| f.to_string()),
            user_id: value.user_id().map(|f| f.to_string()),
            session_id: value.session_id().map(|f| f.to_string()),
            environment: value.environment().map(|f| f.to_string()),
            app_name: value.app_name().map(|f| f.to_string()),
            current_time: value.current_time().map(|f| f.to_string()),
            remote_address: value.remote_address().map(|f| f.to_string()),
            properties: value.properties().map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| Some((entry.key().to_string(), entry.value()?.to_string())))
                    .collect::<HashMap<String, String>>()
            }),
            external_results: value.custom_strategies_results().map(|entries| {
                entries
                    .iter()
                    .map(|entry| (entry.key().to_string(), entry.value()))
                    .collect::<HashMap<String, bool>>()
            }),
        };

        Ok(context)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn flat_new_engine(start_time: i64) -> u32 {
    let start_time = DateTime::from_timestamp_millis(start_time).unwrap();
    let engine = EngineState::initial_state(start_time);
    let engine_ref = Arc::new(Mutex::new(engine));
    Arc::into_raw(engine_ref) as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flat_free_engine(engine_ptr: *const u32) {
    if engine_ptr.is_null() {
        return;
    }
    // the stack pop here drops the last reference to the Arc,
    // which will in turn drop the Mutex and the EngineState
    unsafe { Arc::from_raw(engine_ptr as *const RawPointerDataType) };
}

unsafe fn get_engine(engine_ptr: *const u32) -> Result<ManagedEngine, FlatError> {
    if engine_ptr.is_null() {
        return Err(FlatError::InvalidPointer);
    }
    let arc_instance = unsafe { Arc::from_raw(engine_ptr as *const RawPointerDataType) };

    let cloned_arc = arc_instance.clone();
    forget(arc_instance);

    Ok(cloned_arc)
}

fn recover_lock<T>(lock: &Mutex<T>) -> MutexGuard<T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn local_alloc(len: usize) -> i32 {
    let layout = Layout::from_size_align(len, 1).unwrap();
    let ptr = unsafe { alloc(layout) };
    ptr as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn local_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let layout = Layout::from_size_align(len, 1).unwrap();
        unsafe { dealloc(ptr as *mut u8, layout) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dealloc_response_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, len, len);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_cstring(ptr: u32) {
    if ptr == 0 {
        return;
    }

    unsafe {
        use std::ffi::CString;
        let _ = CString::from_raw(ptr as *mut i8);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn flat_take_state(engine_ptr: u32, json_ptr: u32, json_len: u32) {
    let lock = unsafe { get_engine(engine_ptr as *const u32).unwrap() };
    let mut engine = recover_lock(&lock);

    let json_str = unsafe {
        let json_bytes = slice::from_raw_parts(json_ptr as *const u8, json_len as usize);
        str::from_utf8(json_bytes)
    };

    let Ok(json_str) = json_str else {
        return;
    };

    if let Ok(client_features) = serde_json::from_str::<UpdateMessage>(json_str) {
        engine.take_state(client_features);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn flat_get_state(engine_ptr: u32) -> u32 {
    let lock = unsafe { get_engine(engine_ptr as *const u32).unwrap() };
    let engine = recover_lock(&lock);

    let state = engine.get_state();
    if let Ok(json_str) = serde_json::to_string(&state) {
        use std::ffi::CString;
        let c_string = CString::new(json_str).unwrap();
        return c_string.into_raw() as u32;
    }

    0 // Return null pointer only if serialization failed (shouldn't happen)
}

#[unsafe(no_mangle)]
pub extern "C" fn flat_check_enabled(engine_ptr: i32, message_ptr: i32, message_len: i32) -> u64 {
    let enabled: Result<ResponseMessage<bool>, FlatError> = (|| {
        let bytes =
            unsafe { std::slice::from_raw_parts(message_ptr as *const u8, message_len as usize) };
        let ctx =
            root::<ContextMessage>(bytes).map_err(|e| FlatError::InvalidContext(e.to_string()))?;

        let context: EnrichedContext = ctx
            .try_into()
            .map_err(|e: FlatError| FlatError::InvalidContext(e.to_string()))?;

        let lock = unsafe { get_engine(engine_ptr as *const u32).unwrap() };
        let engine = recover_lock(&lock);

        let enabled = engine.check_enabled(&context);
        let impression_data = engine.should_emit_impression_event(&context.toggle_name);
        engine.count_toggle(&context.toggle_name, enabled.unwrap_or(false));

        Ok(ResponseMessage {
            message: enabled,
            impression_data,
        })
    })();

    Response::build_response(enabled)
}

#[unsafe(no_mangle)]
pub extern "C" fn flat_check_variant(engine_ptr: i32, message_ptr: i32, message_len: i32) -> u64 {
    let extended_variant: Result<ResponseMessage<ExtendedVariantDef>, FlatError> = (|| {
        let bytes =
            unsafe { std::slice::from_raw_parts(message_ptr as *const u8, message_len as usize) };
        let ctx: ContextMessage =
            root::<ContextMessage>(bytes).map_err(|e| FlatError::InvalidContext(e.to_string()))?;

        let context: EnrichedContext = ctx
            .try_into()
            .map_err(|e: FlatError| FlatError::InvalidContext(e.to_string()))?;

        let lock = unsafe { get_engine(engine_ptr as *const u32).unwrap() };
        let engine = recover_lock(&lock);

        let variant = engine.check_variant(&context);
        let enabled = engine.check_enabled(&context).unwrap_or_default();

        engine.count_toggle(&context.toggle_name, enabled);

        if let Some(variant) = &variant {
            engine.count_variant(&context.toggle_name, &variant.name);
        }

        let impression_data = engine.should_emit_impression_event(&context.toggle_name);
        let variant_def = variant.map(|variant| ExtendedVariantDef {
            enabled: variant.enabled,
            feature_enabled: enabled,
            name: variant.name,
            payload: variant.payload.clone(),
        });

        Ok(ResponseMessage {
            message: variant_def,
            impression_data,
        })
    })();

    Variant::build_response(extended_variant)
}

#[unsafe(no_mangle)]
pub extern "C" fn flat_get_metrics(engine_ptr: i32, close_time: i64) -> u64 {
    let lock = unsafe { get_engine(engine_ptr as *const u32).unwrap() };
    let mut engine = recover_lock(&lock);

    let metrics = engine.get_metrics(DateTime::from_timestamp_millis(close_time).unwrap());

    MetricsResponse::build_response(metrics)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flat_list_known_toggles(engine_ptr: i32) -> u64 {
    let lock = unsafe { get_engine(engine_ptr as *const u32).unwrap() };
    let engine = recover_lock(&lock);

    let known_toggles = engine.list_known_toggles();

    FeatureDefs::build_response(known_toggles)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flat_get_core_version() -> u64 {
    CoreVersion::build_response(env!("CARGO_PKG_VERSION"))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flat_get_built_in_strategies() -> u64 {
    BuiltInStrategies::build_response(KNOWN_STRATEGIES)
}
