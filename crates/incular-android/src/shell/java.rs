//! Calls from Rust into the static methods of `dev.incular.IncularNative`,
//! always on the main thread.
//!
//! Strings cross as UTF-16 (`NewString`, `GetStringRegion`): JNI's "UTF"
//! functions use Java's modified UTF-8, which encodes emoji as surrogate
//! pairs that are not valid UTF-8.

use jni_sys::{JNI_OK, JNI_VERSION_1_6, JNIEnv, JavaVM, jboolean, jclass, jint, jstring, jvalue};
use std::{
    ffi::{CStr, c_void},
    sync::atomic::{AtomicPtr, Ordering},
};

static VM: AtomicPtr<JavaVM> = AtomicPtr::new(std::ptr::null_mut());
/// A global reference to `dev.incular.IncularNative`, made at load time on
/// the main thread, whose class loader can find it.
static CLASS: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Remembers the VM and the `IncularNative` class for later calls.
///
/// # Safety
///
/// `vm` and `env` are valid, `env` belongs to the calling thread and
/// `class` is a live reference to `dev.incular.IncularNative`.
pub(super) unsafe fn remember(vm: *mut JavaVM, env: *mut JNIEnv, class: jclass) {
    // SAFETY: guaranteed by the caller; the reference lives as long as the
    // process, like the class.
    let global = unsafe {
        let Some(new_global_ref) = (**env).NewGlobalRef else {
            return;
        };
        new_global_ref(env, class)
    };
    VM.store(vm, Ordering::Release);
    CLASS.store(global.cast(), Ordering::Release);
}

/// The calling thread's JNI environment.
fn env() -> Option<*mut JNIEnv> {
    let vm = VM.load(Ordering::Acquire);
    if vm.is_null() {
        return None;
    }
    let mut env: *mut c_void = std::ptr::null_mut();
    // SAFETY: `vm` was valid when stored and lives for the process; every
    // caller runs on the main thread, which the VM attached.
    let status = unsafe { ((**vm).GetEnv?)(vm, &raw mut env, JNI_VERSION_1_6) };
    (status == JNI_OK && !env.is_null()).then(|| env.cast())
}

/// A Java argument.
pub(super) enum Arg<'a> {
    Int(i32),
    Bool(bool),
    Str(&'a str),
    OptionalStr(Option<&'a str>),
}

/// What a static method returns.
enum Returns {
    Void,
    Bool,
    String,
}

enum Value {
    Void,
    Bool(bool),
    String(Option<String>),
}

/// Calls `IncularNative.<name>` with JNI signature `signature`. A Java
/// exception is logged and cleared, and the call reports nothing.
fn call(name: &CStr, signature: &CStr, args: &[Arg<'_>], returns: Returns) -> Option<Value> {
    let env = env()?;
    let class: jclass = CLASS.load(Ordering::Acquire).cast();
    if class.is_null() {
        return None;
    }
    // SAFETY: `env` is this thread's environment and `class` a live global
    // reference; every function comes from the env's table, local references
    // made here are deleted before returning, and the argument array
    // outlives the call.
    unsafe {
        let table = &**env;
        let method = (table.GetStaticMethodID?)(env, class, name.as_ptr(), signature.as_ptr());
        if method.is_null() {
            clear_exception(env);
            return None;
        }
        let mut locals = Vec::new();
        let values: Vec<jvalue> = args
            .iter()
            .map(|arg| match *arg {
                Arg::Int(value) => jvalue { i: value },
                Arg::Bool(value) => jvalue {
                    z: jboolean::from(value),
                },
                Arg::OptionalStr(None) => jvalue { l: std::ptr::null_mut() },
                Arg::Str(value) | Arg::OptionalStr(Some(value)) => {
                    let string = new_string(env, value);
                    locals.push(string);
                    jvalue { l: string }
                }
            })
            .collect();
        let value = match returns {
            Returns::Void => {
                (table.CallStaticVoidMethodA?)(env, class, method, values.as_ptr());
                Value::Void
            }
            Returns::Bool => Value::Bool(
                (table.CallStaticBooleanMethodA?)(env, class, method, values.as_ptr()) != 0,
            ),
            Returns::String => {
                let string = (table.CallStaticObjectMethodA?)(env, class, method, values.as_ptr());
                let text = string_from_java(env, string);
                locals.push(string);
                Value::String(text)
            }
        };
        if let Some(delete) = table.DeleteLocalRef {
            for local in locals.into_iter().filter(|local| !local.is_null()) {
                delete(env, local);
            }
        }
        if clear_exception(env) {
            return None;
        }
        Some(value)
    }
}

pub(super) fn call_void(name: &CStr, signature: &CStr, args: &[Arg<'_>]) {
    let _ = call(name, signature, args, Returns::Void);
}

pub(super) fn call_bool(name: &CStr, signature: &CStr, args: &[Arg<'_>]) -> bool {
    matches!(
        call(name, signature, args, Returns::Bool),
        Some(Value::Bool(true))
    )
}

pub(super) fn call_string(name: &CStr, signature: &CStr, args: &[Arg<'_>]) -> Option<String> {
    match call(name, signature, args, Returns::String)? {
        Value::String(text) => text,
        Value::Void | Value::Bool(_) => None,
    }
}

/// Logs and clears a pending Java exception; returns whether there was one.
///
/// # Safety
///
/// `env` is the calling thread's environment.
unsafe fn clear_exception(env: *mut JNIEnv) -> bool {
    // SAFETY: guaranteed by the caller.
    unsafe {
        let table = &**env;
        let pending = table.ExceptionCheck.is_some_and(|check| check(env) != 0);
        if pending {
            if let Some(describe) = table.ExceptionDescribe {
                describe(env);
            }
            if let Some(clear) = table.ExceptionClear {
                clear(env);
            }
        }
        pending
    }
}

/// A new local `java.lang.String`, or null on failure.
///
/// # Safety
///
/// `env` is the calling thread's environment.
unsafe fn new_string(env: *mut JNIEnv, text: &str) -> jstring {
    let units: Vec<u16> = text.encode_utf16().collect();
    // SAFETY: guaranteed by the caller; `units` outlives the call.
    unsafe {
        (**env).NewString.map_or(std::ptr::null_mut(), |new| {
            new(env, units.as_ptr(), units.len() as jint)
        })
    }
}

/// Copies a Java string, or returns `None` for null or on a JNI failure.
///
/// # Safety
///
/// `env` is the calling thread's environment and `string` null or a valid
/// `java.lang.String` reference.
pub(super) unsafe fn string_from_java(env: *mut JNIEnv, string: jstring) -> Option<String> {
    if string.is_null() {
        return None;
    }
    // SAFETY: guaranteed by the caller; the buffer holds `length` units.
    unsafe {
        let table = &**env;
        let length = (table.GetStringLength?)(env, string);
        let mut units = vec![0_u16; usize::try_from(length).ok()?];
        (table.GetStringRegion?)(env, string, 0, length, units.as_mut_ptr());
        Some(String::from_utf16_lossy(&units))
    }
}
