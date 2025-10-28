use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jlong, jobject, jstring};
use jni::JNIEnv;

use std::ffi::{c_void, c_char, CString, CStr};
use std::ptr::NonNull;

use crate::flat::serialisation::Buf;

// === C ABI from your library ===
extern "C" {
    fn new_engine() -> *mut c_void;
    fn free_engine(engine_ptr: *mut c_void);

    fn flat_take_state(engine_ptr: *mut c_void, toggles_ptr: *const c_char) -> Buf;
    fn flat_check_enabled(engine_ptr: *mut c_void, msg_ptr: u64, msg_len: u64) -> Buf;
    fn flat_check_variant(engine_ptr: *mut c_void, msg_ptr: u64, msg_len: u64) -> Buf;
    fn flat_list_known_toggles(engine_ptr: *mut c_void) -> Buf;
    fn flat_built_in_strategies() -> Buf;
    fn flat_get_metrics(engine_ptr: *mut c_void) -> Buf;
    fn flat_buf_free(buf: Buf);

    // Optional; if you don’t have it, we’ll fall back to Cargo version
    fn get_core_version() -> *const c_char;
}

// Wrap a Buf as a Java DirectByteBuffer (do NOT free here)
unsafe fn wrap_buf<'a>(env: &mut JNIEnv<'a>, b: Buf) -> jobject {
    // Expect cap == len invariant
    if b.len == 0 {
        // If you may produce empty buffers, either:
        // - return a small non-null 0-len direct buffer, or
        // - handle on Java side by returning null or an empty heap buffer.
        // Easiest here: create a 0-sized direct buffer is not supported by JNI,
        // so just return an empty heap ByteBuffer on the Java side if you need that.
        // For simplicity, assert no empty outputs:
        debug_assert_eq!(b.cap, 0);
        panic!("Zero-length Buf not supported; avoid producing empty outputs or special-case in Java");
    }
    let ptr = NonNull::new(b.ptr).expect("Buf.ptr null");
    env.new_direct_byte_buffer(ptr.as_ptr(), b.len)
        .expect("new_direct_byte_buffer failed")
        .into_raw()
}

// ===== JNI: engine lifecycle =====
#[no_mangle]
pub extern "system" fn Java_io_getunleash_engine_NativeBridge_newEngine(
    _env: JNIEnv,
    _cls: JClass,
) -> jlong {
    unsafe { new_engine() as jlong }
}

#[no_mangle]
pub extern "system" fn Java_io_getunleash_engine_NativeBridge_freeEngine(
    _env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
) {
    if engine_ptr != 0 {
        unsafe { free_engine(engine_ptr as *mut c_void) }
    }
}

// ===== JNI: state update & queries =====
#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatTakeState(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
    toggles_json: JString,
) -> jobject {
    let s = env.get_string(&toggles_json).expect("read jstring");
    let c = CString::new(s.to_bytes()).expect("CString");
    let b = unsafe { flat_take_state(engine_ptr as *mut c_void, c.as_ptr()) };
    wrap_buf(&mut env, b)
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatCheckEnabled(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
    ctx: JByteBuffer,
    len: jlong,
) -> jobject {
    let addr = env.get_direct_buffer_address(&ctx).expect("ctx addr");
    let b = unsafe {
        flat_check_enabled(
            engine_ptr as *mut c_void,
            addr as u64,
            len as u64,
        )
    };
    wrap_buf(&mut env, b)
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatCheckVariant(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
    ctx: JByteBuffer,
    len: jlong,
) -> jobject {
    let addr = env.get_direct_buffer_address(&ctx).expect("ctx addr");
    let b = unsafe {
        flat_check_variant(
            engine_ptr as *mut c_void,
            addr as u64,
            len as u64,
        )
    };
    wrap_buf(&mut env, b)
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatListKnownToggles(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
) -> jobject {
    let b = unsafe { flat_list_known_toggles(engine_ptr as *mut c_void) };
    wrap_buf(&mut env, b)
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatBuiltInStrategies(
    mut env: JNIEnv,
    _cls: JClass,
) -> jobject {
    let b = unsafe { flat_built_in_strategies() };
    wrap_buf(&mut env, b)
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatGetMetrics(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
) -> jobject {
    let b = unsafe { flat_get_metrics(engine_ptr as *mut c_void) };
    wrap_buf(&mut env, b)
}

// ===== JNI: version =====
#[no_mangle]
pub extern "system" fn Java_io_getunleash_engine_NativeBridge_getCoreVersion(
    env: JNIEnv,
    _cls: JClass,
) -> jstring {
    unsafe {
        let p = get_core_version();
        if !p.is_null() {
            let s = CStr::from_ptr(p);
            return env.new_string(s.to_string_lossy()).unwrap().into_raw();
        }
    }
    env.new_string(env!("CARGO_PKG_VERSION")).unwrap().into_raw()
}

// ===== JNI: free returned buffers (cap == len) =====
#[no_mangle]
pub extern "system" fn Java_io_getunleash_engine_NativeBridge_flatBufFree(
    env: JNIEnv,
    _cls: JClass,
    bb: JByteBuffer,
) {
    if let Ok(addr) = env.get_direct_buffer_address(&bb) {
        let len = env.get_direct_buffer_capacity(&bb).unwrap_or(0);
        if len == 0 { return; }
        let b = Buf { ptr: addr, len, cap: len };
        unsafe { flat_buf_free(b) };
    }
}
