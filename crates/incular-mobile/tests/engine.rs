//! The mobile engine binds the first window, refuses later ones, and builds
//! the runtime environment from what the native shell reports. None of this
//! needs a GPU: surfaces are only created when the shell reports one.

use incular_config::{EdgeInsets, Locale};
use incular_core::{Color, Size};
use incular_mobile::{
    MobileEngine, MobileEnvironment, install_application, mobile_platform_capabilities,
    take_application,
};
use incular_platform::{CapabilitySupport, PhysicalSize, WindowMetrics, WindowOptions};
use incular_runtime::{Application, RuntimeWake};
use incular_widgets::Widget;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct CountingWake(AtomicUsize);

impl RuntimeWake for CountingWake {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

fn application() -> Application {
    Application::new(|_| Widget::box_(Size::new(40., 40.), Color::WHITE)).expect("application")
}

fn engine() -> MobileEngine {
    MobileEngine::new(application(), Arc::new(CountingWake::default()))
}

#[test]
fn first_window_binds_to_the_native_view() {
    let mut engine = engine();
    let id = engine
        .window_id()
        .expect("the first window is bound at start");
    assert!(engine.application_mut().contains_window(id));
    let capabilities = engine
        .application_mut()
        .window_capabilities(id)
        .expect("bound windows have capabilities");
    assert_eq!(capabilities, mobile_platform_capabilities());
    assert_eq!(
        capabilities.window.request_redraw,
        CapabilitySupport::Supported
    );
}

#[test]
fn later_windows_are_refused_and_closed() {
    let mut engine = engine();
    let first = engine.window_id().unwrap();
    let second = engine
        .application_mut()
        .open_window(
            WindowOptions::default(),
            Widget::box_(Size::new(10., 10.), Color::BLACK),
        )
        .expect("the runtime accepts the request")
        .id();
    engine.process_runtime_work();
    assert_eq!(engine.window_id(), Some(first));
    assert!(engine.application_mut().contains_window(first));
    assert!(!engine.application_mut().contains_window(second));
}

#[test]
fn frames_wait_for_a_surface() {
    let mut engine = engine();
    assert!(!engine.wants_frame());
    assert!(!engine.frame(incular_core::time::Instant::now()));
}

#[test]
fn environment_snapshot_merges_insets_and_defaults() {
    let french: Locale = "fr-FR".parse().unwrap();
    let environment = MobileEnvironment {
        view_padding: EdgeInsets {
            left: 0.,
            top: 24.,
            right: 0.,
            bottom: 16.,
        },
        view_insets: EdgeInsets {
            left: 0.,
            top: 0.,
            right: 0.,
            bottom: 300.,
        },
        text_scale: Some(1.3),
        locales: Some(vec![french.clone()]),
        ..MobileEnvironment::default()
    };
    let snapshot = environment.snapshot(WindowMetrics::new(PhysicalSize::new(1080, 2400), 2.5));
    assert_eq!(snapshot.viewport, Size::new(432., 960.));
    assert_eq!(snapshot.physical_width, 1080);
    assert_eq!(snapshot.scale_factor, 2.5);
    assert_eq!(snapshot.safe_insets.top, 24.);
    // The keyboard covers the 16 px bottom padding, so no safe bottom is left.
    assert_eq!(snapshot.safe_insets.bottom, 0.);
    assert_eq!(snapshot.view_padding.bottom, 16.);
    assert!((snapshot.text_scale - 1.3).abs() < 1e-6);
    assert_eq!(snapshot.locales.first(), Some(&french));
    assert!(snapshot.input.touch);
    assert!(snapshot.window_focused);
}

#[test]
fn installed_application_is_taken_once() {
    assert!(take_application().is_none());
    install_application(application());
    assert!(take_application().is_some());
    assert!(take_application().is_none());
}
