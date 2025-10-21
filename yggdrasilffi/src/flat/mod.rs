use flatbuffers::root;
use std::ffi::c_void;

use messaging::messaging::{
    ContextMessage, Response,
};
use serialisation::{FlatError, FlatMessage, ResponseMessage};

use crate::flat::serialisation::Buf;
use crate::{get_engine, recover_lock};
use std::collections::HashMap;
use unleash_yggdrasil::state::EnrichedContext;

mod serialisation;

#[allow(clippy::all)]
mod messaging {
    #![allow(dead_code)]
    #![allow(non_snake_case)]
    #![allow(warnings)]
    include!("enabled-message_generated.rs");
}

impl TryFrom<ContextMessage<'_>> for EnrichedContext {
    type Error = FlatError;

    fn try_from(value: ContextMessage) -> Result<Self, Self::Error> {
        let toggle_name = value.toggle_name().ok_or(FlatError::MissingFlagName)?;

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

#[no_mangle]
pub extern "C" fn flat_buf_free(buf: Buf) {
    if buf.ptr.is_null() { return; }
    unsafe {
        drop(Vec::from_raw_parts(buf.ptr, buf.len, buf.cap))
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn flat_get_state(engine_ptr: u32) -> u32 {
    let lock = unsafe { get_engine(engine_ptr as *mut c_void).unwrap() };
    let engine = recover_lock(&lock);

    let state = engine.get_state();
    if let Ok(json_str) = serde_json::to_string(&state) {
        use std::ffi::CString;
        let c_string = CString::new(json_str).unwrap();
        return c_string.into_raw() as u32;
    }

    0 // Return null pointer only if serialization failed (shouldn't happen)
}

#[no_mangle]
/// safety: should only be called from the same thread that created the engine
pub unsafe extern "C" fn flat_check_enabled(
    engine_ptr: *mut c_void,
    message_ptr: u64,
    message_len: u64,
) -> Buf {
    let enabled: Result<ResponseMessage<bool>, FlatError> = (|| {
        let bytes =
            unsafe { std::slice::from_raw_parts(message_ptr as *const u8, message_len as usize) };
        let ctx =
            root::<ContextMessage>(bytes).map_err(|e| FlatError::InvalidContext(e.to_string()))?;

        let context: EnrichedContext = ctx
            .try_into()
            .map_err(|e: FlatError| FlatError::InvalidContext(e.to_string()))?;

        let lock = get_engine(engine_ptr).unwrap();
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

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;
    use crate::flat::messaging::messaging::ContextMessageBuilder;
    use crate::flat::serialisation::allocate;
    use crate::{free_response, new_engine, take_state};
    use flatbuffers::{FlatBufferBuilder, WIPOffset};
    use unleash_types::client_features::{ClientFeature, ClientFeatures, Strategy};
    extern crate flatbuffers;

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
}
