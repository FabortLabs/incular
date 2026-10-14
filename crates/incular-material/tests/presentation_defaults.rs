use incular_core::Color;
use incular_material::{
    MaterialPointerDevice, MaterialScrollBehavior, OutlineInputBorder, SelectableText,
    UnderlineInputBorder,
};
use incular_widgets::{Border, ScrollPhysics, Widget};

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
fn scroll_behavior_default_keeps_physics_scrollbars_and_device_order() {
    let behavior = MaterialScrollBehavior::default();
    assert_eq!(behavior.get_physics(), ScrollPhysics::clamping());
    assert!(behavior.get_scrollbars());
    assert_eq!(
        behavior.get_drag_devices(),
        &[
            MaterialPointerDevice::Touch,
            MaterialPointerDevice::Mouse,
            MaterialPointerDevice::Trackpad,
            MaterialPointerDevice::Stylus,
        ]
    );
    assert_eq!(behavior, MaterialScrollBehavior::builder().build());
}

#[test]
fn selectable_text_default_matches_its_empty_builder_default() {
    let default = SelectableText::default();
    let built = SelectableText::builder().text(String::new()).build();
    assert_eq!(default, built);

    let _: Widget = default.into();
}
