//! Flutter's `SystemUiOverlayStyle` values and the annotation merge rules
//! from `RenderView._updateSystemChrome`.

use incular_config::Brightness;
use incular_core::Color;
use incular_platform::SystemUiOverlayStyle;

#[test]
fn light_and_dark_carry_flutters_values() {
    let light = SystemUiOverlayStyle::LIGHT;
    assert_eq!(light.status_bar_icon_brightness, Some(Brightness::Light));
    assert_eq!(light.status_bar_brightness, Some(Brightness::Dark));
    assert_eq!(light.system_navigation_bar_color, Some(Color::BLACK));
    assert_eq!(
        light.system_navigation_bar_icon_brightness,
        Some(Brightness::Light)
    );
    assert_eq!(light.status_bar_color, None);

    let dark = SystemUiOverlayStyle::DARK;
    assert_eq!(dark.status_bar_icon_brightness, Some(Brightness::Dark));
    assert_eq!(dark.status_bar_brightness, Some(Brightness::Light));
    assert_eq!(dark.system_navigation_bar_color, Some(Color::BLACK));
    assert_eq!(
        dark.system_navigation_bar_icon_brightness,
        Some(Brightness::Light)
    );

    // A default style names nothing, so it changes nothing.
    assert_eq!(
        SystemUiOverlayStyle::default().status_bar_icon_brightness,
        None
    );
}

#[test]
fn upper_annotation_supplies_status_fields_and_lower_the_navigation_fields() {
    let upper = SystemUiOverlayStyle {
        status_bar_icon_brightness: Some(Brightness::Dark),
        status_bar_color: Some(Color::WHITE),
        system_navigation_bar_color: Some(Color::rgba(1, 2, 3, 255)),
        ..SystemUiOverlayStyle::default()
    };
    let lower = SystemUiOverlayStyle {
        status_bar_icon_brightness: Some(Brightness::Light),
        system_navigation_bar_color: Some(Color::BLACK),
        system_navigation_bar_icon_brightness: Some(Brightness::Light),
        ..SystemUiOverlayStyle::default()
    };
    let merged = SystemUiOverlayStyle::merge_annotations(Some(upper), Some(lower))
        .expect("two annotations merge");
    assert_eq!(merged.status_bar_icon_brightness, Some(Brightness::Dark));
    assert_eq!(merged.status_bar_color, Some(Color::WHITE));
    assert_eq!(merged.system_navigation_bar_color, Some(Color::BLACK));
    assert_eq!(
        merged.system_navigation_bar_icon_brightness,
        Some(Brightness::Light)
    );
}

#[test]
fn a_lone_annotation_supplies_every_field_and_none_supplies_nothing() {
    assert_eq!(
        SystemUiOverlayStyle::merge_annotations(Some(SystemUiOverlayStyle::LIGHT), None),
        Some(SystemUiOverlayStyle::LIGHT)
    );
    assert_eq!(
        SystemUiOverlayStyle::merge_annotations(None, Some(SystemUiOverlayStyle::DARK)),
        Some(SystemUiOverlayStyle::DARK)
    );
    assert_eq!(SystemUiOverlayStyle::merge_annotations(None, None), None);
}
