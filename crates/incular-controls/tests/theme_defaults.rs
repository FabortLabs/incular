use incular_controls::{
    slider::Range,
    theme::{
        CheckboxGroupTheme, ControlMetrics, ControlMotion, DrawerTheme, GroupTheme, MeterTheme,
        MotionTokens, PopupTheme, ProgressTheme, ScrollbarTheme, ToggleGroupTheme,
    },
};

#[test]
fn theme_defaults_keep_representative_size_motion_and_scrollbar_values() {
    let metrics = ControlMetrics::default();
    assert_eq!(metrics.control_height, 32.0);
    assert_eq!(metrics.horizontal_padding, 12.0);
    assert_eq!(metrics.scrollbar_thickness, 8.0);

    let motion = ControlMotion::default();
    assert_eq!(motion.hover_duration_ms, 100);
    assert_eq!(motion.pressed_duration_ms, 50);
    assert!(!motion.reduced_motion);

    assert_eq!(ScrollbarTheme::default().min_thumb_extent, 18.0);
}

#[test]
fn slider_range_default_keeps_full_unit_interval_and_step() {
    let range = Range::default();
    assert_eq!(range.min, 0.0);
    assert_eq!(range.max, 1.0);
    assert_eq!(range.step, 0.01);
}

#[test]
fn theme_aliases_keep_their_underlying_default_tokens() {
    let motion: MotionTokens = ControlMotion::default();
    assert_eq!(motion.focus_duration_ms, 120);

    let meter: MeterTheme = ProgressTheme::default();
    assert_eq!(meter.height, 8.0);

    let checkbox_group: CheckboxGroupTheme = GroupTheme::default();
    assert_eq!(checkbox_group.spacing, 6.0);
    let toggle_group: ToggleGroupTheme = GroupTheme::default();
    assert_eq!(toggle_group.radius, 5.0);

    let drawer: DrawerTheme = PopupTheme::default();
    assert_eq!(drawer.elevation, 8.0);
}
