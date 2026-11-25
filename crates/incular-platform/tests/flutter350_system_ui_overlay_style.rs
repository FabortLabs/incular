use std::hash::{DefaultHasher, Hash, Hasher};

use incular_config::Brightness;
use incular_core::Color;
use incular_platform::SystemUiOverlayStyle;

const NAV_COLOR: Color = Color::rgba(0x12, 0x34, 0x56, 0xff);
const DIVIDER: Color = Color::rgba(0x65, 0x43, 0x21, 0xff);
const STATUS_COLOR: Color = Color::rgba(0xab, 0xcd, 0xef, 0xff);
const NAV_VALUE: &str =
    "Color(alpha: 1.0000, red: 0.0706, green: 0.2039, blue: 0.3373, colorSpace: ColorSpace.sRGB)";
const DIVIDER_VALUE: &str =
    "Color(alpha: 1.0000, red: 0.3961, green: 0.2627, blue: 0.1294, colorSpace: ColorSpace.sRGB)";
const STATUS_VALUE: &str =
    "Color(alpha: 1.0000, red: 0.6706, green: 0.8039, blue: 0.9373, colorSpace: ColorSpace.sRGB)";

fn populated() -> SystemUiOverlayStyle {
    SystemUiOverlayStyle {
        system_navigation_bar_color: Some(NAV_COLOR),
        system_navigation_bar_divider_color: Some(DIVIDER),
        system_navigation_bar_icon_brightness: Some(Brightness::Dark),
        system_navigation_bar_contrast_enforced: Some(false),
        status_bar_color: Some(STATUS_COLOR),
        status_bar_brightness: Some(Brightness::Dark),
        status_bar_icon_brightness: Some(Brightness::Light),
        system_status_bar_contrast_enforced: Some(true),
    }
}

#[test]
fn rendering_default_style_displays_all_eight_fields_as_null() {
    let text = SystemUiOverlayStyle::default().to_string();
    assert!(text.starts_with("SystemUiOverlayStyle#"));
    for name in [
        "systemNavigationBarColor",
        "systemNavigationBarDividerColor",
        "systemNavigationBarIconBrightness",
        "systemNavigationBarContrastEnforced",
        "statusBarColor",
        "statusBarBrightness",
        "statusBarIconBrightness",
        "systemStatusBarContrastEnforced",
    ] {
        assert!(text.contains(&format!("{name}: null")), "{name}");
    }
}

#[test]
fn rendering_populated_style_formats_actual_values() {
    let text = populated().to_string();
    assert!(text.starts_with("SystemUiOverlayStyle#"));
    for (name, value) in [
        ("systemNavigationBarColor", NAV_VALUE),
        ("systemNavigationBarDividerColor", DIVIDER_VALUE),
        ("systemNavigationBarIconBrightness", "Brightness.dark"),
        ("systemNavigationBarContrastEnforced", "false"),
        ("statusBarColor", STATUS_VALUE),
        ("statusBarBrightness", "Brightness.dark"),
        ("statusBarIconBrightness", "Brightness.light"),
        ("systemStatusBarContrastEnforced", "true"),
    ] {
        assert!(text.contains(&format!("{name}: {value}")), "{name}");
    }
}

#[test]
fn equality_and_hash_share_the_same_value_fields() {
    let one = SystemUiOverlayStyle {
        system_navigation_bar_color: Some(NAV_COLOR),
        status_bar_brightness: Some(Brightness::Dark),
        ..Default::default()
    };
    let two = one;
    let three = SystemUiOverlayStyle {
        system_navigation_bar_color: Some(DIVIDER),
        ..one
    };
    let hash = |style: SystemUiOverlayStyle| {
        let mut hasher = DefaultHasher::new();
        style.hash(&mut hasher);
        hasher.finish()
    };
    assert_eq!(one, two);
    assert_ne!(one, three);
    assert_eq!(hash(one), hash(two));
    assert_ne!(hash(one), hash(three));
}

#[test]
fn copies_only_explicit_overrides() {
    let one = SystemUiOverlayStyle {
        system_navigation_bar_color: Some(NAV_COLOR),
        status_bar_brightness: Some(Brightness::Dark),
        system_status_bar_contrast_enforced: Some(true),
        ..Default::default()
    };
    let two = one.copy_with(SystemUiOverlayStyle {
        system_navigation_bar_color: Some(DIVIDER),
        status_bar_icon_brightness: Some(Brightness::Light),
        ..Default::default()
    });
    assert_eq!(two.system_navigation_bar_color, Some(DIVIDER));
    assert_eq!(two.status_bar_brightness, Some(Brightness::Dark));
    assert_eq!(two.system_status_bar_contrast_enforced, Some(true));
    assert_eq!(two.status_bar_icon_brightness, Some(Brightness::Light));
    assert_eq!(two.system_navigation_bar_divider_color, None);
    assert_eq!(
        one.copy_with(SystemUiOverlayStyle {
            system_status_bar_contrast_enforced: Some(false),
            ..Default::default()
        })
        .system_status_bar_contrast_enforced,
        Some(false)
    );
}

#[test]
fn diagnostics_names_values_and_order_match() {
    let style = SystemUiOverlayStyle {
        system_navigation_bar_icon_brightness: Some(Brightness::Light),
        system_navigation_bar_contrast_enforced: Some(true),
        system_status_bar_contrast_enforced: Some(false),
        ..populated()
    };
    assert_eq!(
        style
            .diagnostic_properties()
            .map(|(name, value)| format!("{name}: {value}")),
        [
            format!("systemNavigationBarColor: {NAV_VALUE}"),
            format!("systemNavigationBarDividerColor: {DIVIDER_VALUE}"),
            "systemNavigationBarIconBrightness: Brightness.light".to_owned(),
            "systemNavigationBarContrastEnforced: true".to_owned(),
            format!("statusBarColor: {STATUS_VALUE}"),
            "statusBarBrightness: Brightness.dark".to_owned(),
            "statusBarIconBrightness: Brightness.light".to_owned(),
            "systemStatusBarContrastEnforced: false".to_owned(),
        ]
    );
}
