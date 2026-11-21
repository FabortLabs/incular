//! JNI entry points called by `dev.incular.IncularNative`, all on the main
//! thread.
//!
//! The engine lives in a main-thread `thread_local`: every JNI callback, the
//! Choreographer callback and the looper wake callback run on that thread,
//! and none of them re-enters another while it holds the engine.

use incular_core::{NormalizedPressure, PointerDeviceKind, PointerPhase, time::Instant};
use incular_mobile::{MobileEngine, MobileEnvironment, TouchSample};
use incular_platform::{PhysicalSize, PlatformLifecycle};
use incular_runtime::RuntimeWake;
use jni_sys::{
    JNI_ERR, JNI_OK, JNI_VERSION_1_6, JNIEnv, JNINativeMethod, JavaVM, jboolean, jclass, jfloat,
    jint, jobject, jstring,
};
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString, c_char, c_int, c_long, c_void},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::Arc,
};

unsafe extern "C" {
    /// The C entry point rustc generates for the app's binary. The app is
    /// linked as a shared object (`-shared`), so its ordinary `main` runs
    /// here, inside the activity, and installs the application.
    fn main(argc: c_int, argv: *const *const c_char) -> c_int;
}

struct AndroidShell {
    engine: MobileEngine,
    /// Read end of the wake socket, polled by the main looper.
    wake_reader: UnixStream,
    density: f32,
    environment: MobileEnvironment,
}

thread_local! {
    static SHELL: RefCell<Option<AndroidShell>> = const { RefCell::new(None) };
    static FRAME_POSTED: Cell<bool> = const { Cell::new(false) };
}

fn with_shell(f: impl FnOnce(&mut AndroidShell)) {
    SHELL.with(|shell| {
        if let Some(shell) = shell.borrow_mut().as_mut() {
            f(shell);
        }
    });
    schedule_frame_if_needed();
}

/// Wakes the main looper from Tokio worker threads.
struct LooperWake(UnixStream);

impl RuntimeWake for LooperWake {
    fn wake(&self) {
        // A full socket buffer already guarantees a pending wake.
        let _ = (&self.0).write(&[1]);
    }
}

/// An `ANativeWindow` as a WGPU surface target.
struct AndroidSurface(NativeWindow);

impl HasWindowHandle for AndroidSurface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for AndroidSurface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}

fn schedule_frame_if_needed() {
    let wanted = SHELL.with(|shell| {
        shell
            .borrow()
            .as_ref()
            .is_some_and(|shell| shell.engine.wants_frame())
    });
    if wanted {
        post_frame();
    }
}

fn post_frame() {
    if FRAME_POSTED.replace(true) {
        return;
    }
    // SAFETY: called on the main thread, which has a Looper and therefore a
    // Choreographer; the callback takes no data pointer.
    unsafe {
        let choreographer = ndk_sys::AChoreographer_getInstance();
        if choreographer.is_null() {
            FRAME_POSTED.set(false);
            return;
        }
        #[allow(deprecated)]
        ndk_sys::AChoreographer_postFrameCallback(
            choreographer,
            Some(on_frame),
            std::ptr::null_mut(),
        );
    }
}

unsafe extern "C" fn on_frame(_frame_time_nanos: c_long, _data: *mut c_void) {
    FRAME_POSTED.set(false);
    let again = SHELL.with(|shell| {
        shell
            .borrow_mut()
            .as_mut()
            .is_some_and(|shell| shell.engine.frame(Instant::now()))
    });
    if again {
        post_frame();
    }
}

unsafe extern "C" fn on_wake(_fd: c_int, _events: c_int, _data: *mut c_void) -> c_int {
    with_shell(|shell| {
        let mut buffer = [0_u8; 64];
        while matches!((&shell.wake_reader).read(&mut buffer), Ok(n) if n > 0) {}
        shell.engine.process_runtime_work();
    });
    1
}

fn install_logging() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        redirect_stdio_to_logcat();
        let _ = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .without_time()
            .try_init();
    });
}

/// Android discards stdout and stderr. Pipe both into logcat so `println!`,
/// panics and `tracing` output are visible with `adb logcat -s Incular`.
fn redirect_stdio_to_logcat() {
    let mut fds = [0 as c_int; 2];
    // SAFETY: `fds` has room for the two descriptors `pipe` writes.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    let [read, write] = fds;
    // SAFETY: duplicating a valid pipe descriptor onto the standard streams.
    unsafe {
        libc::dup2(write, libc::STDOUT_FILENO);
        libc::dup2(write, libc::STDERR_FILENO);
        libc::close(write);
    }
    let _ = std::thread::Builder::new()
        .name("incular-logcat".into())
        .spawn(move || {
            use std::{io::BufRead, os::fd::FromRawFd};
            // SAFETY: this thread takes sole ownership of the read end.
            let reader = std::io::BufReader::new(unsafe { std::fs::File::from_raw_fd(read) });
            let tag = c"Incular";
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(text) = CString::new(line) {
                    // SAFETY: both strings are NUL-terminated and outlive the call.
                    unsafe {
                        ndk_sys::__android_log_write(
                            ndk_sys::android_LogPriority::ANDROID_LOG_INFO.0 as c_int,
                            tag.as_ptr(),
                            text.as_ptr(),
                        );
                    }
                }
            }
        });
}

fn device_kind(tool_type: jint) -> PointerDeviceKind {
    // MotionEvent.TOOL_TYPE_* values.
    match tool_type {
        1 => PointerDeviceKind::Touch,
        2 => PointerDeviceKind::Stylus,
        3 => PointerDeviceKind::Mouse,
        4 => PointerDeviceKind::InvertedStylus,
        _ => PointerDeviceKind::Unknown,
    }
}

/// Copies a Java string, or returns `None` for null or on a JNI failure.
///
/// # Safety
///
/// `env` must be the calling thread's JNI environment and `string` null or a
/// valid `java.lang.String` reference.
unsafe fn java_string(env: *mut JNIEnv, string: jstring) -> Option<String> {
    if string.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees `env` and `string`; the UTF chars are
    // released before returning and copied out first.
    unsafe {
        let (Some(get), Some(release)) = ((**env).GetStringUTFChars, (**env).ReleaseStringUTFChars)
        else {
            return None;
        };
        let chars = get(env, string, std::ptr::null_mut());
        if chars.is_null() {
            return None;
        }
        let value = CStr::from_ptr(chars).to_string_lossy().into_owned();
        release(env, string, chars);
        Some(value)
    }
}

/// Starts the app. Returns false when the library exported no application.
extern "system" fn native_create(env: *mut JNIEnv, _class: jclass, files_dir: jstring) -> jboolean {
    install_logging();
    if SHELL.with(|shell| shell.borrow().is_some()) {
        // The activity was recreated; keep the running engine.
        return 1;
    }
    // SAFETY: JNI passes a valid env and a local `String` reference.
    match unsafe { java_string(env, files_dir) } {
        // Android has no `$HOME` project directories; restoration and other
        // application data live under the app's private files directory.
        Some(files_dir) => _ = incular_platform::set_application_data_root(files_dir.into()),
        None => eprintln!("Incular: cannot read the app's files directory"),
    }
    // SAFETY: the app binary's C `main` runs its Rust `main` on this (main)
    // thread; std accepts an empty argument vector.
    unsafe { main(0, std::ptr::null()) };
    let Some(application) = incular_mobile::take_application() else {
        eprintln!("Incular: the app's main returned without calling incular::run");
        return 0;
    };
    let Ok((wake_reader, wake_writer)) = UnixStream::pair() else {
        eprintln!("Incular: cannot create the main-looper wake socket");
        return 0;
    };
    let _ = wake_reader.set_nonblocking(true);
    let _ = wake_writer.set_nonblocking(true);
    // SAFETY: called on the main thread, whose looper outlives the process
    // lifetime of this descriptor; the callback takes no data pointer.
    unsafe {
        let looper = ndk_sys::ALooper_forThread();
        ndk_sys::ALooper_addFd(
            looper,
            wake_reader.as_raw_fd(),
            ndk_sys::ALOOPER_POLL_CALLBACK,
            ndk_sys::ALOOPER_EVENT_INPUT as c_int,
            Some(on_wake),
            std::ptr::null_mut(),
        );
    }
    let engine = MobileEngine::new(application, Arc::new(LooperWake(wake_writer)));
    SHELL.with(|shell| {
        *shell.borrow_mut() = Some(AndroidShell {
            engine,
            wake_reader,
            density: 1.0,
            environment: MobileEnvironment::default(),
        });
    });
    1
}

extern "system" fn native_surface_created(
    env: *mut JNIEnv,
    _class: jclass,
    surface: jobject,
    width: jint,
    height: jint,
    density: jfloat,
) {
    // SAFETY: `env` and `surface` are the valid JNI arguments of this call.
    let Some(window) = (unsafe { NativeWindow::from_surface(env, surface) }) else {
        eprintln!("Incular: ANativeWindow_fromSurface failed");
        return;
    };
    with_shell(|shell| {
        shell.density = density;
        let target = incular_wgpu::WindowSurfaceTarget::new(Arc::new(AndroidSurface(window)));
        shell.engine.surface_created(
            target,
            PhysicalSize::new(width.max(0) as u32, height.max(0) as u32),
            f64::from(density),
        );
    });
}

extern "system" fn native_surface_changed(
    _env: *mut JNIEnv,
    _class: jclass,
    width: jint,
    height: jint,
    density: jfloat,
) {
    with_shell(|shell| {
        shell.density = density;
        shell.engine.surface_changed(
            PhysicalSize::new(width.max(0) as u32, height.max(0) as u32),
            f64::from(density),
        );
    });
}

extern "system" fn native_surface_destroyed(_env: *mut JNIEnv, _class: jclass) {
    with_shell(|shell| shell.engine.surface_destroyed());
}

/// `action`: 0 down, 1 move, 2 up, 3 cancel. Coordinates are physical pixels.
extern "system" fn native_touch(
    _env: *mut JNIEnv,
    _class: jclass,
    pointer: jint,
    action: jint,
    x: jfloat,
    y: jfloat,
    pressure: jfloat,
    tool_type: jint,
) {
    let phase = match action {
        0 => PointerPhase::Down,
        1 => PointerPhase::Move,
        2 => PointerPhase::Up,
        _ => PointerPhase::Cancel,
    };
    with_shell(|shell| {
        shell.engine.touch(TouchSample {
            pointer: u64::from(pointer.unsigned_abs()),
            phase,
            kind: device_kind(tool_type),
            x: f64::from(x),
            y: f64::from(y),
            pressure: NormalizedPressure::new(f64::from(pressure)),
        });
    });
}

/// Physical-pixel insets: system bars plus cutout, then the soft keyboard.
extern "system" fn native_insets(
    _env: *mut JNIEnv,
    _class: jclass,
    left: jint,
    top: jint,
    right: jint,
    bottom: jint,
    ime_bottom: jint,
) {
    with_shell(|shell| {
        let scale = shell.density.max(f32::EPSILON);
        let logical = |value: jint| value.max(0) as f32 / scale;
        let mut environment = shell.environment.clone();
        environment.view_padding = incular_config::EdgeInsets {
            left: logical(left),
            top: logical(top),
            right: logical(right),
            bottom: logical(bottom),
        };
        environment.view_insets = incular_config::EdgeInsets {
            bottom: logical(ime_bottom),
            ..incular_config::EdgeInsets::default()
        };
        shell.environment.clone_from(&environment);
        shell.engine.set_environment(environment);
    });
}

/// `state`: 0 resumed, 1 paused, 2 stopped, 3 started.
extern "system" fn native_lifecycle(_env: *mut JNIEnv, _class: jclass, state: jint) {
    let lifecycle = match state {
        0 => PlatformLifecycle::Active,
        1 => PlatformLifecycle::Inactive,
        2 => PlatformLifecycle::Suspended,
        _ => PlatformLifecycle::Resumed,
    };
    with_shell(|shell| shell.engine.lifecycle(lifecycle));
}

/// Registers the natives of `dev.incular.IncularNative` when the activity
/// loads the app library. Registration (instead of exported `Java_*`
/// symbols) keeps the functions alive in a binary linked as a shared object
/// and leaves Kotlin names free to change with one table.
///
/// # Safety
///
/// Called by the Android runtime with a valid `JavaVM`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn JNI_OnLoad(vm: *mut JavaVM, _reserved: *mut c_void) -> jint {
    fn method(
        name: &'static CStr,
        signature: &'static CStr,
        function: *mut c_void,
    ) -> JNINativeMethod {
        JNINativeMethod {
            name: name.as_ptr().cast_mut(),
            signature: signature.as_ptr().cast_mut(),
            fnPtr: function,
        }
    }
    let methods = [
        method(
            c"nativeCreate",
            c"(Ljava/lang/String;)Z",
            native_create as *mut c_void,
        ),
        method(
            c"nativeSurfaceCreated",
            c"(Landroid/view/Surface;IIF)V",
            native_surface_created as *mut c_void,
        ),
        method(
            c"nativeSurfaceChanged",
            c"(IIF)V",
            native_surface_changed as *mut c_void,
        ),
        method(
            c"nativeSurfaceDestroyed",
            c"()V",
            native_surface_destroyed as *mut c_void,
        ),
        method(c"nativeTouch", c"(IIFFFI)V", native_touch as *mut c_void),
        method(c"nativeInsets", c"(IIIII)V", native_insets as *mut c_void),
        method(c"nativeLifecycle", c"(I)V", native_lifecycle as *mut c_void),
    ];
    // SAFETY: `vm` is valid for the duration of the call; every pointer
    // below comes from the JNI function tables it hands out, and the method
    // table outlives `RegisterNatives`.
    unsafe {
        let mut env: *mut c_void = std::ptr::null_mut();
        let Some(get_env) = (**vm).GetEnv else {
            return JNI_ERR;
        };
        if get_env(vm, &raw mut env, JNI_VERSION_1_6) != JNI_OK {
            return JNI_ERR;
        }
        let env = env.cast::<JNIEnv>();
        let (Some(find_class), Some(register)) = ((**env).FindClass, (**env).RegisterNatives)
        else {
            return JNI_ERR;
        };
        let class = find_class(env, c"dev/incular/IncularNative".as_ptr());
        if class.is_null()
            || register(env, class, methods.as_ptr(), methods.len() as jint) != JNI_OK
        {
            return JNI_ERR;
        }
    }
    JNI_VERSION_1_6
}
