//! The snapshot a mobile shell hands the runtime: safe insets are the
//! persistent padding minus the transient keyboard, and the system settings
//! the shell reports arrive unchanged.

use incular_config::{Brightness, EdgeInsets, Locale};
use incular_mobile::MobileEnvironment;
use incular_platform::{PhysicalSize, WindowMetrics};

fn metrics() -> WindowMetrics {
    WindowMetrics::new(PhysicalSize::new(1080, 2400), 2.0)
}

fn bottom(value: f32) -> EdgeInsets {
    EdgeInsets::only(0.0, 0.0, 0.0, value)
}

#[test]
fn keyboard_taller_than_the_home_indicator_leaves_no_safe_bottom() {
    let environment = MobileEnvironment {
        view_padding: bottom(48.0),
        view_insets: bottom(300.0),
        ..MobileEnvironment::default()
    };

    let snapshot = environment.snapshot(metrics());

    assert_eq!(snapshot.safe_insets.bottom, 0.0);
    assert_eq!(snapshot.view_padding.bottom, 48.0);
    assert_eq!(snapshot.view_insets.bottom, 300.0);
}

#[test]
fn without_a_keyboard_safe_bottom_is_the_padding() {
    let environment = MobileEnvironment {
        view_padding: bottom(48.0),
        ..MobileEnvironment::default()
    };

    assert_eq!(environment.snapshot(metrics()).safe_insets.bottom, 48.0);
}

#[test]
fn keyboard_shorter_than_the_padding_subtracts_from_it() {
    let environment = MobileEnvironment {
        view_padding: bottom(48.0),
        view_insets: bottom(20.0),
        ..MobileEnvironment::default()
    };

    assert_eq!(environment.snapshot(metrics()).safe_insets.bottom, 28.0);
}

#[test]
fn system_settings_reach_the_runtime_environment() {
    let locales = vec![
        "fr-FR".parse::<Locale>().expect("fr-FR"),
        "en-US".parse::<Locale>().expect("en-US"),
    ];
    let environment = MobileEnvironment {
        brightness: Some(Brightness::Dark),
        text_scale: Some(1.3),
        locales: Some(locales.clone()),
        reduced_motion: Some(true),
        bold_text: Some(true),
        ..MobileEnvironment::default()
    };

    let snapshot = environment.snapshot(metrics());

    assert_eq!(snapshot.brightness, Brightness::Dark);
    assert_eq!(snapshot.text_scale, 1.3);
    assert_eq!(snapshot.locales, locales);
    assert!(snapshot.reduced_motion);
    assert!(snapshot.bold_text);
}

#[test]
fn unset_settings_keep_the_runtime_defaults() {
    let defaults = incular_config::RuntimeEnvironment::default();
    let snapshot = MobileEnvironment::default().snapshot(metrics());

    assert_eq!(snapshot.brightness, defaults.brightness);
    assert_eq!(snapshot.text_scale, defaults.text_scale);
    assert_eq!(snapshot.locales, defaults.locales);
    assert_eq!(snapshot.reduced_motion, defaults.reduced_motion);
    assert_eq!(snapshot.bold_text, defaults.bold_text);
}
