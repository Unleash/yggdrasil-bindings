use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jlong, jobject, jstring};
use jni::JNIEnv;

use crate::flat::serialisation::Buf;
use crate::get_state;
use std::ffi::{c_char, c_void, CStr, CString};
use std::panic;
use std::ptr::NonNull;

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

const NATIVE_EX_CLASS: &str = "io/getunleash/engine/NativeException";

#[inline]
fn throw_java(env: &mut JNIEnv, msg: impl AsRef<str>) {
    let _ = env.throw_new(NATIVE_EX_CLASS, msg.as_ref());
}

/// Run a closure, catching Rust panics and mapping errors to Java exceptions.
/// Returns Ok(val) or throws and returns Err(()).
fn jni_guard<T, F>(env: &mut JNIEnv<'_>, f: F) -> Result<T, ()>
where
    F: FnOnce(&mut JNIEnv<'_>) -> Result<T, String>,
{
    match panic::catch_unwind(panic::AssertUnwindSafe(|| f(env))) {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(msg)) => {
            throw_java(env, msg);
            Err(())
        }
        Err(_) => {
            throw_java(env, "native panic");
            Err(())
        }
    }
}

// No panics: turn problems into Java exceptions + return null
unsafe fn wrap_buf(env: &mut JNIEnv<'_>, b: Buf) -> jobject {
    if b.len == 0 {
        throw_java(env, "native returned empty buffer");
        return std::ptr::null_mut();
    }
    let nn = match NonNull::new(b.ptr) {
        Some(p) => p,
        None => {
            throw_java(env, "native buffer pointer was null");
            return std::ptr::null_mut();
        }
    };
    match env.new_direct_byte_buffer(nn.as_ptr(), b.len) {
        Ok(bb) => bb.into_raw(),
        Err(e) => {
            throw_java(env, format!("new_direct_byte_buffer failed: {e}"));
            std::ptr::null_mut()
        }
    }
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
    let res = jni_guard(&mut env, |env| {
        // 1) Read from env
        let js = env
            .get_string(&toggles_json)
            .map_err(|e| format!("get_string: {e}"))?;
        let rust_json: String = js.into(); // proper UTF-16 → UTF-8

        let c = CString::new(rust_json).map_err(|_| "JSON contained NUL byte")?;

        // 2) Native call
        let b = unsafe { flat_take_state(engine_ptr as *mut c_void, c.as_ptr()) };

        // 3) Wrap
        Ok(wrap_buf(env, b))
    });
    res.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatCheckEnabled(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
    ctx: JByteBuffer,
    len: jlong,
) -> jobject {
    let res = jni_guard(&mut env, |env| {
        let addr = env
            .get_direct_buffer_address(&ctx)
            .map_err(|e| format!("get_direct_buffer_address: {e}"))?;
        if len < 0 {
            return Err("negative length".into());
        }

        let b = unsafe { flat_check_enabled(engine_ptr as *mut c_void, addr as u64, len as u64) };
        Ok(wrap_buf(env, b))
    });
    res.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatCheckVariant(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
    ctx: JByteBuffer,
    len: jlong,
) -> jobject {
    let res = jni_guard(&mut env, |env| {
        let addr = env
            .get_direct_buffer_address(&ctx)
            .map_err(|e| format!("get_direct_buffer_address: {e}"))?;
        if len < 0 {
            return Err("negative length".into());
        }

        let b = unsafe { flat_check_variant(engine_ptr as *mut c_void, addr as u64, len as u64) };
        Ok(wrap_buf(env, b))
    });
    res.unwrap_or(std::ptr::null_mut())
}

// List known toggles  ---------------------------------------------------------
#[no_mangle]
pub unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatListKnownToggles(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
) -> jobject {
    let res = jni_guard(&mut env, |env| {
        let b = unsafe { flat_list_known_toggles(engine_ptr as *mut c_void) };
        Ok(wrap_buf(env, b))
    });
    res.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatBuiltInStrategies(
    mut env: JNIEnv,
    _cls: JClass,
) -> jobject {
    let res = jni_guard(&mut env, |env| {
        let b = unsafe { flat_built_in_strategies() };
        Ok(wrap_buf(env, b))
    });
    res.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatGetMetrics(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
) -> jobject {
    let res = jni_guard(&mut env, |env| {
        let b = unsafe { flat_get_metrics(engine_ptr as *mut c_void) };
        Ok(wrap_buf(env, b))
    });
    res.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
unsafe extern "system" fn Java_io_getunleash_engine_NativeBridge_flatGetState(
    mut env: JNIEnv,
    _cls: JClass,
    engine_ptr: jlong,
) -> jstring {
    let res = jni_guard(&mut env, |env| {
        // COMPUTE: fetch C string pointer; handle null
        let ptr = unsafe { get_state(engine_ptr as *mut c_void) };
        if ptr.is_null() {
            return Err("get_state returned null".into());
        }
        let rust_s = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();

        // WRAP: Java String
        let j = env
            .new_string(rust_s)
            .map_err(|e| format!("new_string failed: {e}"))?;
        Ok(j.into_raw())
    });
    res.unwrap_or(std::ptr::null_mut())
}

// ===== JNI: version =====
#[no_mangle]
pub extern "system" fn Java_io_getunleash_engine_NativeBridge_getCoreVersion(
    mut env: JNIEnv,
    _cls: JClass,
) -> jstring {
    let res = jni_guard(&mut env, |env| {
        // COMPUTE: get version string from native (or fallback)
        let ver: String = unsafe {
            let p = get_core_version();
            if !p.is_null() {
                // to_string_lossy never panics; copies into an owned String
                CStr::from_ptr(p).to_string_lossy().into_owned()
            } else {
                env!("CARGO_PKG_VERSION").to_string()
            }
        };

        // WRAP: create Java String
        let jstr = env
            .new_string(ver)
            .map_err(|e| format!("new_string failed: {e}"))?;
        Ok(jstr.into_raw())
    });

    // If an exception was thrown, return null so JVM can continue.
    res.unwrap_or(std::ptr::null_mut())
}

// ===== JNI: free returned buffers (cap == len) =====
#[no_mangle]
pub extern "system" fn Java_io_getunleash_engine_NativeBridge_flatBufFree(
    mut env: JNIEnv,
    _cls: JClass,
    bb: JByteBuffer,
) {
    // READ
    let _ = jni_guard(&mut env, |env| {
        let addr = env
            .get_direct_buffer_address(&bb)
            .map_err(|e| format!("get_direct_buffer_address: {e}"))?;
        let len = env
            .get_direct_buffer_capacity(&bb)
            .map_err(|e| format!("get_direct_buffer_capacity: {e}"))?;

        // COMPUTE
        if len == 0 {
            // 0-length: nothing was allocated (we used a dangling pointer policy),
            // so there's nothing to free. Treat as success.
            return Ok(());
        }

        // cap == len invariant required by your builder
        let b = Buf {
            ptr: addr,
            len,
            cap: len,
        };
        unsafe { flat_buf_free(b) };

        // RETURN (nothing to wrap for void-return JNI)
        Ok(())
    });
    // If an error occurred, a Java exception is already pending; just return.
}
