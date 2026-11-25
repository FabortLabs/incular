//! Flutter's `SystemChrome.setSystemUIOverlayStyle` and annotation-driven
//! system bar styles, as the runtime hands them to the host once per frame.

use incular_config::{Brightness, Constraints, EdgeInsets, RuntimeEnvironment};
use incular_core::Size;
use incular_platform::SystemUiOverlayStyle;
use incular_runtime::Runtime;
use incular_widgets::{AnnotatedRegion, SizedBox, Widget};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(220., 50.))
}

/// A 220x50 view whose status bar is 24 logical pixels tall.
fn runtime(root: Widget) -> Runtime {
    let mut runtime = Runtime::new(root).expect("runtime");
    runtime.set_environment(RuntimeEnvironment {
        viewport: Size::new(220., 50.),
        view_padding: EdgeInsets {
            left: 0.,
            top: 24.,
            right: 0.,
            bottom: 0.,
        },
        ..RuntimeEnvironment::default()
    });
    runtime
}

#[test]
fn an_annotation_over_the_status_bar_styles_it_after_the_frame() {
    let mut runtime = runtime(
        AnnotatedRegion::new(
            SystemUiOverlayStyle::LIGHT,
            SizedBox::expand().child(SizedBox::expand()),
        )
        .into(),
    );
    runtime.run_frame(constraints()).expect("frame");
    let style = runtime
        .take_system_ui_overlay_style()
        .expect("the annotation asks for a style");
    assert_eq!(style.status_bar_icon_brightness, Some(Brightness::Light));
    assert_eq!(style.status_bar_brightness, Some(Brightness::Dark));
}

#[test]
fn an_unchanged_annotation_is_not_reported_again() {
    let mut runtime =
        runtime(AnnotatedRegion::new(SystemUiOverlayStyle::DARK, SizedBox::expand()).into());
    runtime.run_frame(constraints()).expect("frame");
    assert!(runtime.take_system_ui_overlay_style().is_some());
    runtime.run_frame(constraints()).expect("frame");
    assert_eq!(runtime.take_system_ui_overlay_style(), None);
}

#[test]
fn the_imperative_setter_is_drained_once_and_an_annotation_overrides_it() {
    let mut plain = runtime(SizedBox::expand().into());
    plain.set_system_ui_overlay_style(SystemUiOverlayStyle::DARK);
    assert_eq!(
        plain.take_system_ui_overlay_style(),
        Some(SystemUiOverlayStyle::DARK)
    );
    plain.run_frame(constraints()).expect("frame");
    assert_eq!(plain.take_system_ui_overlay_style(), None);

    let mut annotated =
        runtime(AnnotatedRegion::new(SystemUiOverlayStyle::LIGHT, SizedBox::expand()).into());
    annotated.set_system_ui_overlay_style(SystemUiOverlayStyle::DARK);
    annotated.run_frame(constraints()).expect("frame");
    assert_eq!(
        annotated.take_system_ui_overlay_style(),
        Some(SystemUiOverlayStyle::LIGHT)
    );
}

#[test]
fn detached_host_can_receive_the_same_style_again() {
    use incular_runtime::Application;
    let mut app = Application::new(|_| SizedBox::expand().into()).expect("application");
    let id = app.primary_window();
    let style = SystemUiOverlayStyle::default();

    app.set_window_system_ui_overlay_style(id, style);
    assert_eq!(app.take_window_system_ui_overlay_style(id), Some(style));
    app.set_window_system_ui_overlay_style(id, style);
    assert_eq!(app.take_window_system_ui_overlay_style(id), None);

    app.invalidate_applied_window_system_ui_overlay_style(id);
    assert_eq!(app.take_window_system_ui_overlay_style(id), None);
    app.set_window_system_ui_overlay_style(id, style);
    assert_eq!(app.take_window_system_ui_overlay_style(id), Some(style));
    assert_eq!(app.take_window_system_ui_overlay_style(id), None);
}

#[test]
fn invalidating_the_host_preserves_the_latest_pending_request() {
    use incular_runtime::Application;
    let mut app = Application::new(|_| SizedBox::expand().into()).expect("application");
    let id = app.primary_window();
    let light = SystemUiOverlayStyle::LIGHT;
    app.set_window_system_ui_overlay_style(id, light);
    assert_eq!(app.take_window_system_ui_overlay_style(id), Some(light));

    app.set_window_system_ui_overlay_style(id, SystemUiOverlayStyle::DARK);
    app.set_window_system_ui_overlay_style(id, light);
    app.invalidate_applied_window_system_ui_overlay_style(id);
    assert_eq!(app.take_window_system_ui_overlay_style(id), Some(light));
    assert_eq!(app.take_window_system_ui_overlay_style(id), None);
}
