//! UIKit classes and the main-thread engine they drive.
//!
//! `run_application` registers the classes, stores the application and calls
//! `UIApplicationMain`. The generated `Info.plist` names
//! `IncularSceneDelegate` as the scene delegate, which creates the window,
//! the view controller and the Metal-backed `IncularView` for each scene.
//! Phase 1 binds the application's first window to the first scene's view.

use incular_core::{NormalizedPressure, PointerDeviceKind, PointerPhase, time::Instant};
use incular_mobile::{MobileEngine, MobileEnvironment, TouchSample};
use incular_platform::{PhysicalSize, PlatformLifecycle};
use incular_runtime::{Application, RuntimeWake};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{ClassType, DeclaredClass, declare_class, msg_send, msg_send_id, mutability, sel};
use objc2_foundation::{
    CGRect, MainThreadMarker, NSDictionary, NSRunLoop, NSRunLoopCommonModes, NSSet, NSString,
};
use objc2_quartz_core::{CADisplayLink, CAMetalLayer};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey, UIApplicationMain,
    UIEvent, UIResponder, UIScene, UISceneConnectionOptions, UISceneDelegate, UISceneSession,
    UITouch, UITouchPhase, UITouchType, UIView, UIViewController, UIWindow, UIWindowScene,
    UIWindowSceneDelegate,
};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    UiKitWindowHandle, WindowHandle,
};
use std::{
    cell::{Cell, RefCell},
    ffi::{c_char, c_int},
    ptr::NonNull,
    sync::Arc,
};

struct IosShell {
    engine: MobileEngine,
    display_link: Retained<CADisplayLink>,
    view: Option<Retained<IncularView>>,
    surface_bound: bool,
    environment: MobileEnvironment,
}

thread_local! {
    static PENDING: RefCell<Option<Application>> = const { RefCell::new(None) };
    static SHELL: RefCell<Option<IosShell>> = const { RefCell::new(None) };
    static IN_FRAME: Cell<bool> = const { Cell::new(false) };
}

fn with_shell(f: impl FnOnce(&mut IosShell)) {
    SHELL.with(|shell| {
        if let Some(shell) = shell.borrow_mut().as_mut() {
            f(shell);
        }
    });
    update_display_link();
}

/// Runs the display link only while the engine wants frames.
fn update_display_link() {
    if IN_FRAME.get() {
        return;
    }
    SHELL.with(|shell| {
        if let Some(shell) = shell.borrow().as_ref() {
            // SAFETY: main-thread call on a live display link.
            unsafe { shell.display_link.setPaused(!shell.engine.wants_frame()) };
        }
    });
}

/// Hands work completed on Tokio threads to the main thread.
struct MainThreadWake(NonNull<AnyObject>);

// SAFETY: the target is a leaked `IncularFrameTarget` that lives for the rest
// of the process, and `performSelectorOnMainThread:` may be sent from any
// thread.
unsafe impl Send for MainThreadWake {}
// SAFETY: see `Send`; the pointer is never dereferenced in Rust.
unsafe impl Sync for MainThreadWake {}

impl RuntimeWake for MainThreadWake {
    fn wake(&self) {
        // SAFETY: the receiver is alive for the process lifetime and
        // `wake:` takes one object argument, which may be nil.
        unsafe {
            let _: () = msg_send![
                self.0.as_ptr(),
                performSelectorOnMainThread: sel!(wake:),
                withObject: std::ptr::null::<AnyObject>(),
                waitUntilDone: false
            ];
        }
    }
}

/// The Metal-backed view as a WGPU surface target.
struct UiKitSurface(NonNull<IncularView>);

// SAFETY: WGPU only reads the view pointer to find its `CAMetalLayer` while
// creating the surface on the main thread; the shell keeps the view retained
// for as long as the surface exists.
unsafe impl Send for UiKitSurface {}
// SAFETY: see `Send`.
unsafe impl Sync for UiKitSurface {}

impl HasWindowHandle for UiKitSurface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = UiKitWindowHandle::new(self.0.cast());
        // SAFETY: the view outlives the returned borrow (see `Send`).
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::UiKit(handle)) })
    }
}

impl HasDisplayHandle for UiKitSurface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::uikit())
    }
}

declare_class!(
    struct IncularFrameTarget;

    unsafe impl ClassType for IncularFrameTarget {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "IncularFrameTarget";
    }

    impl DeclaredClass for IncularFrameTarget {}

    unsafe impl IncularFrameTarget {
        #[method(step:)]
        fn step(&self, _link: &CADisplayLink) {
            IN_FRAME.set(true);
            SHELL.with(|shell| {
                if let Some(shell) = shell.borrow_mut().as_mut() {
                    shell.engine.frame(Instant::now());
                }
            });
            IN_FRAME.set(false);
            update_display_link();
        }

        #[method(wake:)]
        fn wake(&self, _argument: Option<&AnyObject>) {
            with_shell(|shell| shell.engine.process_runtime_work());
        }
    }
);

declare_class!(
    struct IncularView;

    unsafe impl ClassType for IncularView {
        #[inherits(UIResponder, NSObject)]
        type Super = UIView;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "IncularView";
    }

    impl DeclaredClass for IncularView {}

    unsafe impl IncularView {
        #[method(layerClass)]
        fn layer_class() -> &'static objc2::runtime::AnyClass {
            CAMetalLayer::class()
        }

        #[method(layoutSubviews)]
        fn layout_subviews(&self) {
            // SAFETY: forwarding to UIView's implementation.
            let _: () = unsafe { msg_send![super(self), layoutSubviews] };
            self.report_size();
        }

        #[method(safeAreaInsetsDidChange)]
        fn safe_area_insets_did_change(&self) {
            // SAFETY: forwarding to UIView's implementation.
            let _: () = unsafe { msg_send![super(self), safeAreaInsetsDidChange] };
            let insets = self.safeAreaInsets();
            with_shell(|shell| {
                let mut environment = shell.environment.clone();
                environment.view_padding = incular_config::EdgeInsets {
                    left: insets.left as f32,
                    top: insets.top as f32,
                    right: insets.right as f32,
                    bottom: insets.bottom as f32,
                };
                shell.environment.clone_from(&environment);
                shell.engine.set_environment(environment);
            });
        }

        #[method(touchesBegan:withEvent:)]
        fn touches_began(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.forward_touches(touches);
        }

        #[method(touchesMoved:withEvent:)]
        fn touches_moved(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.forward_touches(touches);
        }

        #[method(touchesEnded:withEvent:)]
        fn touches_ended(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.forward_touches(touches);
        }

        #[method(touchesCancelled:withEvent:)]
        fn touches_cancelled(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            self.forward_touches(touches);
        }
    }
);

impl IncularView {
    fn new(mtm: MainThreadMarker, frame: CGRect) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        // SAFETY: `initWithFrame:` is UIView's designated initializer.
        let view: Retained<Self> = unsafe { msg_send_id![super(this), initWithFrame: frame] };
        view.setMultipleTouchEnabled(true);
        view
    }

    fn report_size(&self) {
        let bounds = self.bounds();
        let scale = self.contentScaleFactor();
        let size = PhysicalSize::new(
            (bounds.size.width * scale).round().max(0.0) as u32,
            (bounds.size.height * scale).round().max(0.0) as u32,
        );
        let pointer = NonNull::from(self);
        with_shell(|shell| {
            if shell.surface_bound {
                shell.engine.surface_changed(size, scale);
            } else if size.width > 0 && size.height > 0 {
                shell.surface_bound = true;
                let target =
                    incular_wgpu::WindowSurfaceTarget::new(Arc::new(UiKitSurface(pointer)));
                shell.engine.surface_created(target, size, scale);
            }
        });
    }

    fn forward_touches(&self, touches: &NSSet<UITouch>) {
        let scale = self.contentScaleFactor();
        let samples: Vec<TouchSample> = touches
            .iter()
            .map(|touch| {
                let location = touch.locationInView(Some(self));
                let phase = match touch.phase() {
                    UITouchPhase::Began => PointerPhase::Down,
                    UITouchPhase::Ended => PointerPhase::Up,
                    UITouchPhase::Cancelled => PointerPhase::Cancel,
                    _ => PointerPhase::Move,
                };
                let kind = if touch.r#type() == UITouchType::Pencil {
                    PointerDeviceKind::Stylus
                } else {
                    PointerDeviceKind::Touch
                };
                TouchSample {
                    // A touch object keeps its identity for the whole contact.
                    pointer: std::ptr::from_ref::<UITouch>(touch) as usize as u64,
                    phase,
                    kind,
                    x: location.x * scale,
                    y: location.y * scale,
                    pressure: NormalizedPressure::from_range(
                        touch.force(),
                        touch.maximumPossibleForce(),
                    ),
                }
            })
            .collect();
        with_shell(|shell| {
            for sample in samples {
                shell.engine.touch(sample);
            }
        });
    }
}

declare_class!(
    struct IncularAppDelegate;

    unsafe impl ClassType for IncularAppDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "IncularAppDelegate";
    }

    impl DeclaredClass for IncularAppDelegate {}

    unsafe impl NSObjectProtocol for IncularAppDelegate {}

    unsafe impl UIApplicationDelegate for IncularAppDelegate {
        #[method(application:didFinishLaunchingWithOptions:)]
        fn did_finish_launching(
            &self,
            _application: &UIApplication,
            _options: Option<&NSDictionary<UIApplicationLaunchOptionsKey, AnyObject>>,
        ) -> bool {
            start_engine(MainThreadMarker::from(self));
            true
        }
    }
);

declare_class!(
    struct IncularSceneDelegate;

    unsafe impl ClassType for IncularSceneDelegate {
        #[inherits(NSObject)]
        type Super = UIResponder;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "IncularSceneDelegate";
    }

    impl DeclaredClass for IncularSceneDelegate {
        type Ivars = RefCell<Option<Retained<UIWindow>>>;
    }

    unsafe impl NSObjectProtocol for IncularSceneDelegate {}

    unsafe impl UISceneDelegate for IncularSceneDelegate {
        #[method(scene:willConnectToSession:options:)]
        fn will_connect(
            &self,
            scene: &UIScene,
            _session: &UISceneSession,
            _options: &UISceneConnectionOptions,
        ) {
            let mtm = MainThreadMarker::from(self);
            // SAFETY: the application role in the generated scene manifest
            // only creates window scenes.
            let scene = unsafe { &*std::ptr::from_ref(scene).cast::<UIWindowScene>() };
            // SAFETY: `initWithWindowScene:` is UIWindow's scene initializer.
            let window = unsafe { UIWindow::initWithWindowScene(mtm.alloc(), scene) };
            let view = IncularView::new(mtm, window.bounds());
            // SAFETY: plain main-thread view controller construction.
            let controller = unsafe { UIViewController::new(mtm) };
            controller.setView(Some(&view));
            window.setRootViewController(Some(&controller));
            window.makeKeyAndVisible();
            SHELL.with(|shell| {
                if let Some(shell) = shell.borrow_mut().as_mut()
                    && shell.view.is_none()
                {
                    shell.view = Some(view);
                }
            });
            *self.ivars().borrow_mut() = Some(window);
        }

        #[method(sceneDidBecomeActive:)]
        fn did_become_active(&self, _scene: &UIScene) {
            with_shell(|shell| shell.engine.lifecycle(PlatformLifecycle::Active));
        }

        #[method(sceneWillResignActive:)]
        fn will_resign_active(&self, _scene: &UIScene) {
            with_shell(|shell| shell.engine.lifecycle(PlatformLifecycle::Inactive));
        }

        #[method(sceneWillEnterForeground:)]
        fn will_enter_foreground(&self, _scene: &UIScene) {
            with_shell(|shell| shell.engine.lifecycle(PlatformLifecycle::Resumed));
        }

        #[method(sceneDidEnterBackground:)]
        fn did_enter_background(&self, _scene: &UIScene) {
            with_shell(|shell| shell.engine.lifecycle(PlatformLifecycle::Suspended));
        }
    }

    unsafe impl UIWindowSceneDelegate for IncularSceneDelegate {
        #[method_id(window)]
        fn window(&self) -> Option<Retained<UIWindow>> {
            self.ivars().borrow().clone()
        }

        #[method(setWindow:)]
        fn set_window(&self, window: Option<&UIWindow>) {
            *self.ivars().borrow_mut() = window.map(|window| window.retain());
        }
    }
);

fn start_engine(_mtm: MainThreadMarker) {
    let Some(application) = PENDING.with(|pending| pending.borrow_mut().take()) else {
        return;
    };
    let target: Retained<IncularFrameTarget> =
        // SAFETY: plain NSObject subclass with no ivars.
        unsafe { msg_send_id![IncularFrameTarget::alloc(), init] };
    let wake = MainThreadWake(NonNull::from(&*target).cast());
    // SAFETY: the target is retained by the display link and leaked below;
    // `step:` matches the display link callback signature.
    let display_link = unsafe {
        let link = CADisplayLink::displayLinkWithTarget_selector(&target, sel!(step:));
        link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
        link.setPaused(true);
        link
    };
    // The wake target must outlive every Tokio wake.
    std::mem::forget(target);
    let engine = MobileEngine::new(application, Arc::new(wake));
    SHELL.with(|shell| {
        *shell.borrow_mut() = Some(IosShell {
            engine,
            display_link,
            view: None,
            surface_bound: false,
            environment: MobileEnvironment::default(),
        });
    });
}

/// Registers the UIKit classes and runs `UIApplicationMain`, which never
/// returns.
pub(crate) fn run(application: Application) -> ! {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .without_time()
        .try_init();
    MainThreadMarker::new().expect("incular::run must be called on the main thread");
    PENDING.with(|pending| *pending.borrow_mut() = Some(application));
    // UIKit looks classes up by name, so they must be registered first.
    let _ = IncularSceneDelegate::class();
    let _ = IncularView::class();
    let _ = IncularFrameTarget::class();
    let delegate = NSString::from_str(IncularAppDelegate::NAME);
    unsafe extern "C" {
        fn _NSGetArgc() -> *mut c_int;
        fn _NSGetArgv() -> *mut *mut *mut c_char;
    }
    // SAFETY: the process arguments come from the C runtime and outlive the
    // call; the delegate class is registered above.
    unsafe {
        let _ = IncularAppDelegate::class();
        UIApplicationMain(
            *_NSGetArgc(),
            NonNull::new(*_NSGetArgv()).expect("argv"),
            None,
            Some(&delegate),
        );
    }
    std::process::exit(0)
}
