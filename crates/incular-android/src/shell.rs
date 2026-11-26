//! JNI entry points called by `dev.incular.IncularNative`, all on the main
//! thread.
//!
//! The engine lives in a main-thread `thread_local`: every JNI callback, the
//! Choreographer callback and the looper wake callback run on that thread,
//! and none of them re-enters another while it holds the engine.

use incular_config::Brightness;
use incular_core::{
    NormalizedPressure, PointerDeviceKind, PointerPhase, SECONDARY_POINTER_BUTTON, StylusMetadata,
    StylusOrientation, TERTIARY_POINTER_BUTTON,
    time::{Duration, Instant},
};
use incular_mobile::{MobileEngine, MobileEnvironment, ScrollSample, TouchSample};
use incular_platform::{
    PhysicalSize, PlatformLifecycle, SystemUiOverlayStyle, TextInputCommand,
    canonicalize_system_locales,
};
use incular_runtime::RuntimeWake;
use jni_sys::{
    JNI_ERR, JNI_OK, JNI_VERSION_1_6, JNIEnv, JNINativeMethod, JavaVM, jboolean, jclass, jfloat,
    jint, jlong, jobject, jstring,
};
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::{
    cell::{Cell, RefCell},
    f32::consts::{FRAC_PI_2, TAU},
    ffi::{CStr, CString, c_char, c_int, c_long, c_void},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::Arc,
};

mod java;
mod system_chrome;
mod text_input;

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
    /// The last system bar, cutout and keyboard insets in physical pixels
    /// (`left, top, right, bottom, ime_bottom`). They are kept unscaled because
    /// the activity reports them before the surface reports the density.
    physical_insets: [jint; 5],
    environment: MobileEnvironment,
}

impl AndroidShell {
    /// Converts the physical insets with the current density and republishes
    /// the environment.
    fn publish_insets(&mut self) {
        let scale = self.density.max(f32::EPSILON);
        let logical = |value: jint| value.max(0) as f32 / scale;
        let [left, top, right, bottom, ime_bottom] = self.physical_insets;
        self.environment.view_padding = incular_config::EdgeInsets {
            left: logical(left),
            top: logical(top),
            right: logical(right),
            bottom: logical(bottom),
        };
        self.environment.view_insets = incular_config::EdgeInsets {
            bottom: logical(ime_bottom),
            ..incular_config::EdgeInsets::default()
        };
        self.engine.set_environment(self.environment.clone());
    }
}

thread_local! {
    static SHELL: RefCell<Option<AndroidShell>> = const { RefCell::new(None) };
    static FRAME_POSTED: Cell<bool> = const { Cell::new(false) };
}

/// The platform requests a shell drains after running the engine: text input
/// commands and the system bar style.
#[derive(Default)]
struct PlatformRequests {
    text_input: Vec<TextInputCommand>,
    system_ui_overlay_style: Option<SystemUiOverlayStyle>,
}

impl PlatformRequests {
    fn take(shell: &mut AndroidShell) -> Self {
        Self {
            text_input: shell.engine.take_text_input_commands(),
            system_ui_overlay_style: shell.engine.take_system_ui_overlay_style(),
        }
    }

    /// Applies them outside the engine borrow: Kotlin may call back in.
    fn apply(self) {
        text_input::apply(self.text_input);
        if let Some(style) = self.system_ui_overlay_style {
            system_chrome::apply(style);
        }
    }
}

/// Runs `f` on the engine, then applies the text input commands it queued
/// (outside the borrow: Kotlin may call back in) and schedules a frame.
fn with_shell(f: impl FnOnce(&mut AndroidShell)) {
    let requests = SHELL.with(|shell| {
        shell
            .borrow_mut()
            .as_mut()
            .map_or_else(PlatformRequests::default, |shell| {
                f(shell);
                PlatformRequests::take(shell)
            })
    });
    requests.apply();
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
    let (again, requests) = SHELL.with(|shell| {
        shell.borrow_mut().as_mut().map_or_else(
            || (false, PlatformRequests::default()),
            |shell| {
                let again = shell.engine.frame(Instant::now());
                (again, PlatformRequests::take(shell))
            },
        )
    });
    requests.apply();
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

/// Logs `tracing` events to logcat unless the app's `main` installed its own
/// subscriber, as on iOS and desktop where the app owns logging.
fn install_fallback_subscriber() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .without_time()
        .try_init();
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

/// Publishes the JVM and the application context through `ndk-context`, where
/// Rust crates that call into Android (opening URLs, audio, certificate
/// verification) look for them, and initializes rustls-platform-verifier when
/// the `platform-verifier` feature is on.
///
/// # Safety
///
/// `env` is the calling thread's valid JNI env and `context` a live local
/// reference to an `android.content.Context`.
unsafe fn publish_android_context(env: *mut JNIEnv, context: jobject) {
    // SAFETY: the caller's `env` is valid; both functions come from its table.
    let (vm, global) = unsafe {
        let mut vm: *mut JavaVM = std::ptr::null_mut();
        let (Some(get_java_vm), Some(new_global_ref)) = ((**env).GetJavaVM, (**env).NewGlobalRef)
        else {
            return;
        };
        if get_java_vm(env, &raw mut vm) != JNI_OK || context.is_null() {
            eprintln!("Incular: cannot reach the JavaVM or the application context");
            return;
        }
        // A process-lifetime global reference: the application context
        // outlives every activity, and ndk-context hands it out forever.
        (vm, new_global_ref(env, context))
    };
    // SAFETY: `vm` and `global` are valid for the life of the process, and
    // this runs once (see `native_create`).
    unsafe { ndk_context::initialize_android_context(vm.cast(), global.cast()) };
    #[cfg(feature = "platform-verifier")]
    {
        use jni::errors::LogErrorAndDefault;
        use jni::objects::JObject;
        // SAFETY: as above; the env belongs to this thread for the call.
        let mut unowned = unsafe { jni::EnvUnowned::from_raw(env.cast()) };
        unowned
            .with_env(|env| {
                // SAFETY: `global` is a live global reference to a Context.
                let context = unsafe { JObject::from_raw(env, global.cast()) };
                rustls_platform_verifier::android::init_with_env(env, context)
            })
            .resolve::<LogErrorAndDefault>();
    }
}

/// Starts the app. Returns false when the library exported no application.
extern "system" fn native_create(
    env: *mut JNIEnv,
    _class: jclass,
    context: jobject,
    files_dir: jstring,
) -> jboolean {
    static STDIO: std::sync::Once = std::sync::Once::new();
    STDIO.call_once(|| {
        redirect_stdio_to_logcat();
        // SAFETY: JNI passes a valid env and a local `Context` reference;
        // `Once` keeps the process-wide registration to one call.
        unsafe { publish_android_context(env, context) };
    });
    if SHELL.with(|shell| shell.borrow().is_some()) {
        // The activity was recreated; keep the running engine.
        return 1;
    }
    // SAFETY: JNI passes a valid env and a local `String` reference.
    match unsafe { java::string_from_java(env, files_dir) } {
        // Android has no `$HOME` project directories; restoration and other
        // application data live under the app's private files directory.
        Some(files_dir) => _ = incular_platform::set_application_data_root(files_dir.into()),
        None => eprintln!("Incular: cannot read the app's files directory"),
    }
    // SAFETY: the app binary's C `main` runs its Rust `main` on this (main)
    // thread; std accepts an empty argument vector.
    unsafe { main(0, std::ptr::null()) };
    install_fallback_subscriber();
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
    let mut engine = MobileEngine::new(application, Arc::new(LooperWake(wake_writer)));
    engine.set_clipboard(Box::new(text_input::AndroidClipboard));
    SHELL.with(|shell| {
        *shell.borrow_mut() = Some(AndroidShell {
            engine,
            wake_reader,
            density: 1.0,
            physical_insets: [0; 5],
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
        shell.publish_insets();
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
        shell.publish_insets();
    });
}

extern "system" fn native_surface_destroyed(_env: *mut JNIEnv, _class: jclass) {
    with_shell(|shell| shell.engine.surface_destroyed());
}

/// `action`: 0 down, 1 move, 2 up, 3 cancel, 4 hover enter, 5 hover exit.
/// Coordinates are physical pixels. `buttons` is the pressed-button mask and
/// `changed_button` the button that went down or up (0 when none).
/// `age_nanos` is how long ago the platform recorded the sample. `tilt` and
/// `orientation` are stylus axes in radians, NaN when the tool is not a stylus.
#[allow(
    clippy::too_many_arguments,
    reason = "mirrors the JNI signature of IncularNative.nativeTouch"
)]
extern "system" fn native_touch(
    _env: *mut JNIEnv,
    _class: jclass,
    pointer: jint,
    device: jint,
    action: jint,
    x: jfloat,
    y: jfloat,
    pressure: jfloat,
    tool_type: jint,
    buttons: jint,
    changed_button: jint,
    age_nanos: jlong,
    tilt: jfloat,
    orientation: jfloat,
) {
    let phase = match action {
        0 => PointerPhase::Down,
        1 => PointerPhase::Move,
        2 => PointerPhase::Up,
        4 => PointerPhase::Enter,
        5 => PointerPhase::Exit,
        _ => PointerPhase::Cancel,
    };
    let buttons = u32::try_from(buttons).unwrap_or(0);
    let kind = device_kind(tool_type);
    let stylus = matches!(
        kind,
        PointerDeviceKind::Stylus | PointerDeviceKind::InvertedStylus
    )
    .then(|| stylus_metadata(tilt, orientation, buttons));
    let sample = TouchSample {
        pointer: u64::from(pointer.unsigned_abs()),
        device: u64::from(device as u32),
        phase,
        kind,
        x: f64::from(x),
        y: f64::from(y),
        pressure: NormalizedPressure::new(f64::from(pressure)),
        buttons,
        button: u32::try_from(changed_button)
            .ok()
            .filter(|button| *button != 0),
        stylus,
        age: duration_from_nanos(age_nanos),
    };
    with_shell(|shell| shell.engine.touch(sample));
}

/// `x`, `y` is the pointer position; `dx`, `dy` the scroll in physical pixels
/// (positive toward the bottom and right).
extern "system" fn native_scroll(
    _env: *mut JNIEnv,
    _class: jclass,
    x: jfloat,
    y: jfloat,
    dx: jfloat,
    dy: jfloat,
    age_nanos: jlong,
) {
    with_shell(|shell| {
        shell.engine.scroll(ScrollSample {
            x: f64::from(x),
            y: f64::from(y),
            delta_x: f64::from(dx),
            delta_y: f64::from(dy),
            age: duration_from_nanos(age_nanos),
        });
    });
}

fn duration_from_nanos(nanos: jlong) -> Duration {
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(0))
}

/// Stylus tilt (zero perpendicular to the screen) and orientation (zero
/// pointing up the screen) as Incular's altitude and azimuth. The barrel
/// buttons arrive as the secondary and tertiary bits.
fn stylus_metadata(tilt: f32, orientation: f32, buttons: u32) -> StylusMetadata {
    StylusMetadata {
        orientation: (tilt.is_finite() && orientation.is_finite()).then(|| StylusOrientation {
            altitude: (FRAC_PI_2 - tilt).clamp(0.0, FRAC_PI_2),
            azimuth: (orientation - FRAC_PI_2).rem_euclid(TAU),
        }),
        barrel_button: buttons & (SECONDARY_POINTER_BUTTON | TERTIARY_POINTER_BUTTON) != 0,
    }
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
        shell.physical_insets = [left, top, right, bottom, ime_bottom];
        shell.publish_insets();
    });
}

/// The system settings `IncularActivity` reports: dark mode, font scale, bold
/// text, reduced motion, and comma-separated BCP-47 locale tags in preference
/// order. Tags that do not parse are skipped; no usable tag keeps the default.
extern "system" fn native_settings(
    env: *mut JNIEnv,
    _class: jclass,
    dark: jboolean,
    text_scale: jfloat,
    bold_text: jboolean,
    reduced_motion: jboolean,
    locales: jstring,
) {
    // SAFETY: JNI passes this thread's environment and a valid String reference.
    let tags = unsafe { java::string_from_java(env, locales) }.unwrap_or_default();
    let locales = canonicalize_system_locales(tags.split(','));
    with_shell(|shell| {
        let mut environment = shell.environment.clone();
        environment.brightness = Some(if dark != 0 {
            Brightness::Dark
        } else {
            Brightness::Light
        });
        environment.text_scale = Some(text_scale);
        environment.bold_text = Some(bold_text != 0);
        environment.reduced_motion = Some(reduced_motion != 0);
        environment.locales = (!locales.is_empty()).then_some(locales);
        shell.environment.clone_from(&environment);
        shell.engine.set_environment(environment);
    });
}

/// `state`: 0 resumed, 1 paused, 2 stopped. Like Flutter, the app is active
/// from `onResume` only; `onStart` sends nothing. Unknown values are ignored.
extern "system" fn native_lifecycle(_env: *mut JNIEnv, _class: jclass, state: jint) {
    let lifecycle = match state {
        0 => PlatformLifecycle::Active,
        1 => PlatformLifecycle::Inactive,
        2 => PlatformLifecycle::Suspended,
        _ => return,
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
            c"(Landroid/content/Context;Ljava/lang/String;)Z",
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
        method(
            c"nativeTouch",
            c"(IIIFFFIIIJFF)V",
            native_touch as *mut c_void,
        ),
        method(c"nativeScroll", c"(FFFFJ)V", native_scroll as *mut c_void),
        method(c"nativeInsets", c"(IIIII)V", native_insets as *mut c_void),
        method(
            c"nativeSettings",
            c"(ZFZZLjava/lang/String;)V",
            native_settings as *mut c_void,
        ),
        method(c"nativeLifecycle", c"(I)V", native_lifecycle as *mut c_void),
        method(
            c"nativeEditingState",
            c"(Ljava/lang/String;IIII)V",
            text_input::native_editing_state as *mut c_void,
        ),
        method(
            c"nativeEditorAction",
            c"(I)V",
            text_input::native_editor_action as *mut c_void,
        ),
        method(
            c"nativeKey",
            c"(ZIIIZII)Z",
            text_input::native_key as *mut c_void,
        ),
        method(
            c"nativeBack",
            c"()Z",
            text_input::native_back as *mut c_void,
        ),
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
        java::remember(vm, env, class);
    }
    JNI_VERSION_1_6
}
