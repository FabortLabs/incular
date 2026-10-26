//! Platform accessibility preferences reach `MediaQueryData` and reset to
//! Flutter's defaults when a backend stops reporting them.

use incular_config::RuntimeEnvironment;
use incular_platform::SystemEnvironmentPreferences;

#[test]
fn reported_preferences_apply_and_unsupported_ones_reset() {
    let mut environment = RuntimeEnvironment::default();
    SystemEnvironmentPreferences {
        always_use_24_hour_format: Some(true),
        accessible_navigation: Some(true),
        invert_colors: Some(true),
        on_off_switch_labels: Some(true),
        bold_text: Some(true),
        ..SystemEnvironmentPreferences::default()
    }
    .apply_to(&mut environment);
    assert!(environment.always_use_24_hour_format);
    assert!(environment.accessible_navigation);
    assert!(environment.invert_colors);
    assert!(environment.on_off_switch_labels);
    assert!(environment.bold_text);

    SystemEnvironmentPreferences::default().apply_to(&mut environment);
    assert!(!environment.always_use_24_hour_format);
    assert!(!environment.accessible_navigation);
    assert!(!environment.invert_colors);
    assert!(!environment.on_off_switch_labels);
    assert!(!environment.bold_text);
}
