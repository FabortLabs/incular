use incular_config::TargetPlatform;
use incular_core::Color;
use incular_core::PointerDeviceKind;
use incular_material::{
    MaterialPointerDevice, MaterialScrollBehavior, OutlineInputBorder, SelectableText,
    UnderlineInputBorder,
};
use incular_widgets::{AndroidOverscrollIndicator, Border, ScrollPhysics, Widget};

#[test]
fn input_border_defaults_keep_the_material_outline_and_underline_values() {
    let border = Border::new(1.0, Color::rgba(121, 116, 126, 255));

    let outline = OutlineInputBorder::default();
    assert_eq!(outline.border, border);
    assert_eq!(outline.radius, 4.0);
    assert_eq!(outline, OutlineInputBorder::builder().build());

    let underline = UnderlineInputBorder::default();
    assert_eq!(underline.border, border);
    assert_eq!(underline.radius, 0.0);
    assert_eq!(underline, UnderlineInputBorder::builder().build());
}

#[test]
fn scroll_behavior_defaults_follow_flutter_material_scroll_behavior() {
    // Flutter's MaterialScrollBehavior keeps the base ScrollBehavior's
    // platform physics and touch-like drag devices: no mouse dragging.
    let behavior = MaterialScrollBehavior::default();
    assert!(behavior.get_scrollbars());
    assert_eq!(
        behavior.get_drag_devices(),
        &[
            MaterialPointerDevice::Touch,
            MaterialPointerDevice::Stylus,
            MaterialPointerDevice::Trackpad,
            MaterialPointerDevice::Unknown,
        ]
    );
    assert_eq!(behavior, MaterialScrollBehavior::builder().build());
    let widgets = behavior.scroll_behavior();
    assert!(!widgets.accepts_drag(PointerDeviceKind::Mouse));
    assert!(widgets.accepts_drag(PointerDeviceKind::InvertedStylus));
    assert_eq!(
        behavior
            .clone()
            .platform(TargetPlatform::Android)
            .get_physics(),
        ScrollPhysics::clamping().range_maintaining()
    );
    assert_eq!(
        behavior.clone().platform(TargetPlatform::Ios).get_physics(),
        ScrollPhysics::clamping().bouncing().range_maintaining()
    );
    assert_eq!(
        behavior
            .clone()
            .physics(ScrollPhysics::clamping())
            .get_physics(),
        ScrollPhysics::clamping()
    );
    // buildOverscrollIndicator: Android stretches under Material 3.
    let android = behavior.clone().platform(TargetPlatform::Android);
    assert_eq!(
        android.overscroll_indicator(true),
        Some(AndroidOverscrollIndicator::Stretch)
    );
    assert_eq!(
        android.overscroll_indicator(false),
        Some(AndroidOverscrollIndicator::Glow)
    );
    assert_eq!(
        behavior
            .clone()
            .platform(TargetPlatform::Fuchsia)
            .overscroll_indicator(true),
        Some(AndroidOverscrollIndicator::Glow)
    );
    assert_eq!(
        behavior
            .platform(TargetPlatform::Linux)
            .overscroll_indicator(true),
        None
    );
}

#[test]
fn selectable_text_default_matches_its_empty_builder_default() {
    let default = SelectableText::default();
    let built = SelectableText::builder().text(String::new()).build();
    assert_eq!(default, built);

    let _: Widget = default.into();
}
