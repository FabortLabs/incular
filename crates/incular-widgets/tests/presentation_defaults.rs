use std::time::Duration;

use incular_core::Color;
use incular_widgets::{
    BorderSide, BorderStyle, RawScrollbarOrientation, RawScrollbarStyle, RawTooltipDurations,
};

#[test]
fn border_side_default_preserves_public_values() {
    assert_eq!(
        BorderSide::default(),
        BorderSide {
            color: Color::BLACK,
            width: 1.0,
            style: BorderStyle::Solid,
            stroke_align: -1.0,
        }
    );
}

#[test]
fn raw_scrollbar_style_default_preserves_public_values() {
    assert_eq!(
        RawScrollbarStyle::default(),
        RawScrollbarStyle {
            thickness: 6.0,
            min_thumb_extent: 18.0,
            min_overscroll_thumb_extent: 18.0,
            main_axis_margin: 0.0,
            cross_axis_margin: 0.0,
            track_color: Color::TRANSPARENT,
            thumb_color: Color::rgba(188, 188, 188, 102),
            orientation: RawScrollbarOrientation::Right,
            interactive: true,
            thumb_visibility: None,
            fade_duration: Duration::from_millis(300),
            time_to_fade: Duration::from_millis(600),
        }
    );
}

#[test]
fn raw_tooltip_durations_default_preserves_public_values() {
    let durations = RawTooltipDurations::default();
    assert_eq!(durations.wait_duration, Duration::ZERO);
    assert_eq!(durations.show_duration, Duration::from_millis(1_500));
    assert_eq!(durations.exit_duration, Duration::from_millis(100));
}
