use incular_devtools_protocol::{DebugProperty, DebugValue, DevWidgetId};
use incular_devtools_ui::property_edit::*;
use std::time::{Duration, Instant};

fn property(name: &str, value: DebugValue) -> DebugProperty {
    DebugProperty {
        name: name.into(),
        value,
        overridden: false,
        editable: true,
        options: Vec::new(),
        min: None,
        max: None,
        step: None,
    }
}

fn options(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn hex_colours_parse_with_and_without_alpha() {
    assert_eq!(parse_color("#FF8000"), Some((255, 128, 0, 255)));
    assert_eq!(parse_color("ff8000"), Some((255, 128, 0, 255)));
    assert_eq!(parse_color("#26A69ACC"), Some((0x26, 0xA6, 0x9A, 0xCC)));
    assert_eq!(parse_color("#F80"), Some((255, 136, 0, 255)));
    assert_eq!(parse_color("#F808"), Some((255, 136, 0, 136)));
    assert_eq!(parse_color("  #000000  "), Some((0, 0, 0, 255)));
    for bad in [
        "", "#", "#12", "#12345", "#GG0000", "#1234567", "red", "#ff 000",
    ] {
        assert_eq!(parse_color(bad), None, "{bad:?}");
    }
}

#[test]
fn colours_format_opaque_short_and_translucent_long() {
    assert_eq!(format_color(255, 128, 0, 255), "#FF8000");
    assert_eq!(format_color(0x26, 0xA6, 0x9A, 0xCC), "#26A69ACC");
    assert_eq!(format_color(0, 0, 0, 0), "#00000000");
    for color in [(1, 2, 3, 255), (9, 8, 7, 6), (255, 255, 255, 0)] {
        let (r, g, b, a) = color;
        assert_eq!(parse_color(&format_color(r, g, b, a)), Some(color));
    }
}

#[test]
fn floats_format_without_noise_and_parse_strictly() {
    assert_eq!(format_float(24.), "24");
    assert_eq!(format_float(0.15000000000000002), "0.15");
    assert_eq!(format_float(-0.0), "0");
    assert_eq!(format_float(1.5), "1.5");
    assert_eq!(format_float(0.0004), "0");
    assert_eq!(parse_float(" 12.5 "), Some(12.5));
    assert_eq!(parse_float("∞"), None);
    assert_eq!(parse_float(""), None);
    assert_eq!(parse_float("NaN"), None);
    assert_eq!(parse_float("inf"), None);
}

#[test]
fn clamping_respects_either_or_both_bounds() {
    assert_eq!(clamp_to(5., Some(0.), Some(1.)), 1.);
    assert_eq!(clamp_to(-5., Some(0.), Some(1.)), 0.);
    assert_eq!(clamp_to(5., None, Some(1.)), 1.);
    assert_eq!(clamp_to(-5., Some(0.), None), 0.);
    assert_eq!(clamp_to(0.3, None, None), 0.3);
}

#[test]
fn stepping_snaps_to_the_grid_clamps_and_scales_with_shift() {
    let (min, max) = (Some(0.), Some(1.));
    let mut value = 0.;
    for _ in 0..3 {
        value = step_value(value, 1., 0.05, false, min, max);
    }
    assert_eq!(value, 0.15, "no float drift after repeated steps");
    assert_eq!(step_value(0.95, 1., 0.05, false, min, max), 1.);
    assert_eq!(step_value(1., 1., 0.05, false, min, max), 1.);
    assert_eq!(step_value(0., -1., 0.05, false, min, max), 0.);
    assert_eq!(step_value(10., 1., 1., true, None, None), 20.);
    assert_eq!(step_value(10., -1., 0.5, true, Some(8.), None), 8.);
    // A degenerate step falls back to 1.
    assert_eq!(step_value(3., 1., 0., false, None, None), 4.);
    // Off-grid values snap to the grid.
    assert_eq!(step_value(3.2, 1., 1., false, None, None), 4.);
}

#[test]
fn linked_insets_set_every_side_and_unlinked_one() {
    let current = [1., 2., 3., 4.];
    assert_eq!(
        apply_inset_edit(current, 2, 9., true, Some(0.), Some(200.)),
        [9.; 4]
    );
    assert_eq!(
        apply_inset_edit(current, 2, 9., false, Some(0.), Some(200.)),
        [1., 2., 9., 4.]
    );
    assert_eq!(
        apply_inset_edit(current, 0, 999., false, Some(0.), Some(200.)),
        [200., 2., 3., 4.]
    );
    assert_eq!(
        apply_inset_edit(current, 1, -4., true, Some(0.), Some(200.)),
        [0.; 4]
    );
    assert!(insets_uniform([8.; 4]));
    assert!(!insets_uniform([8., 8., 8., 9.]));
}

#[test]
fn alignment_grid_maps_names_to_cells_and_back() {
    let all = options(&[
        "TopLeft",
        "TopCenter",
        "TopRight",
        "CenterLeft",
        "Center",
        "CenterRight",
        "BottomLeft",
        "BottomCenter",
        "BottomRight",
    ]);
    assert!(is_alignment_grid(&all));
    assert!(!is_alignment_grid(&all[..8]));
    for row in 0..3 {
        for column in 0..3 {
            let name = alignment_name(column, row);
            assert_eq!(alignment_cell(name), Some((column, row)), "{name}");
            assert!(all.iter().any(|option| option == name));
        }
    }
    assert_eq!(alignment_cell("Start"), None);
}

#[test]
fn editor_kind_follows_value_shape_and_metadata() {
    let mut size = property("fontSize", DebugValue::Float(24.));
    size.min = Some(1.);
    size.max = Some(200.);
    size.step = Some(1.);
    assert_eq!(
        editor_kind(&size),
        EditorKind::Float {
            min: Some(1.),
            max: Some(200.),
            step: 1.
        }
    );
    assert_eq!(
        editor_kind(&property("color", DebugValue::Color(0, 0, 0, 255))),
        EditorKind::Color
    );
    assert_eq!(
        editor_kind(&property("visible", DebugValue::Bool(true))),
        EditorKind::Bool
    );
    assert_eq!(
        editor_kind(&property("text", DebugValue::Str("x".into()))),
        EditorKind::Text
    );
    assert_eq!(
        editor_kind(&property("padding", DebugValue::Insets([0.; 4]))),
        EditorKind::Insets {
            min: None,
            max: None,
            step: 1.
        }
    );
    assert_eq!(
        editor_kind(&property("count", DebugValue::Int(3))),
        EditorKind::Raw
    );
    assert_eq!(
        editor_kind(&property(
            "background",
            DebugValue::Optional(Some(Box::new(DebugValue::Color(1, 2, 3, 4))))
        )),
        EditorKind::OptionalColor
    );
    assert_eq!(
        editor_kind(&property("background", DebugValue::Optional(None))),
        EditorKind::OptionalColor
    );
    let unbounded = property("maxWidth", DebugValue::Optional(None));
    assert!(matches!(
        editor_kind(&unbounded),
        EditorKind::OptionalFloat { .. }
    ));
}

#[test]
fn enum_options_choose_segmented_menu_or_grid() {
    let mut size = property("mainAxisSize", DebugValue::Enum("Max".into()));
    size.options = options(&["Min", "Max"]);
    assert_eq!(
        editor_kind(&size),
        EditorKind::Segmented(options(&["Min", "Max"]))
    );
    let mut four = property("x", DebugValue::Enum("A".into()));
    four.options = options(&["A", "B", "C", "D"]);
    assert!(matches!(editor_kind(&four), EditorKind::Segmented(_)));
    let mut five = property("crossAxisAlignment", DebugValue::Enum("Start".into()));
    five.options = options(&["Start", "End", "Center", "Stretch", "Baseline"]);
    assert!(matches!(editor_kind(&five), EditorKind::Menu(_)));
    let mut grid = property("alignment", DebugValue::Enum("Center".into()));
    grid.options = options(&[
        "TopLeft",
        "TopCenter",
        "TopRight",
        "CenterLeft",
        "Center",
        "CenterRight",
        "BottomLeft",
        "BottomCenter",
        "BottomRight",
    ]);
    assert!(matches!(editor_kind(&grid), EditorKind::AlignmentGrid(_)));
    // An enum without advertised options falls back to raw text.
    assert_eq!(
        editor_kind(&property("x", DebugValue::Enum("A".into()))),
        EditorKind::Raw
    );
    assert_eq!(PALETTE.len(), 12);
    assert!(PALETTE.iter().all(|hex| parse_color(hex).is_some()));
}

#[test]
fn optional_values_expose_their_number_and_colour() {
    let some = DebugValue::Optional(Some(Box::new(DebugValue::Float(120.5))));
    assert_eq!(float_text(&some).as_deref(), Some("120.5"));
    assert_eq!(float_of(&some), Some(120.5));
    assert_eq!(
        float_text(&DebugValue::Optional(None)).as_deref(),
        Some("∞")
    );
    assert_eq!(float_of(&DebugValue::Optional(None)), None);
    assert_eq!(float_text(&DebugValue::Bool(true)), None);
    let color = DebugValue::Optional(Some(Box::new(DebugValue::Color(1, 2, 3, 4))));
    assert_eq!(color_of(&color), Some((1, 2, 3, 4)));
    assert_eq!(color_of(&DebugValue::Optional(None)), None);
}

#[test]
fn originals_record_once_and_drive_revert_and_modified_state() {
    let id = DevWidgetId::new(7, 0);
    let key = property_key(id, "fontSize");
    assert_ne!(key, property_key(id, "color"));
    let mut originals = OriginalValues::default();
    assert!(originals.is_empty());
    assert_eq!(originals.revert_value(&key), None);
    // Unknown originals: trust the target's overridden flag.
    assert!(originals.is_modified(&key, true, &DebugValue::Float(30.)));
    assert!(!originals.is_modified(&key, false, &DebugValue::Float(30.)));

    originals.record(&key, &DebugValue::Float(24.));
    // Later edits do not replace the first-seen original.
    originals.record(&key, &DebugValue::Float(27.));
    assert_eq!(originals.original(&key), Some(&DebugValue::Float(24.)));
    assert_eq!(originals.revert_value(&key), Some(DebugValue::Float(24.)));
    assert!(originals.is_modified(&key, true, &DebugValue::Float(27.)));
    // Back at the original (even if the target still says overridden).
    assert!(!originals.is_modified(&key, true, &DebugValue::Float(24.0000001)));

    originals.clear();
    assert!(originals.is_empty());
    assert_eq!(originals.revert_value(&key), None);
}

#[test]
fn values_equal_tolerates_numeric_noise_only() {
    assert!(values_equal(
        &DebugValue::Insets([1., 2., 3., 4.]),
        &DebugValue::Insets([1., 2., 3., 4.00001])
    ));
    assert!(!values_equal(
        &DebugValue::Insets([1., 2., 3., 4.]),
        &DebugValue::Insets([1., 2., 3., 5.])
    ));
    assert!(!values_equal(
        &DebugValue::Optional(None),
        &DebugValue::Optional(Some(Box::new(DebugValue::Float(1.))))
    ));
    assert!(values_equal(
        &DebugValue::Enum("A".into()),
        &DebugValue::Enum("A".into())
    ));
}

#[test]
fn throttle_sends_first_edit_then_coalesces_to_the_latest() {
    let mut throttle = EditThrottle::new(Duration::from_millis(30));
    let start = Instant::now();
    let float = |value: f64| DebugValue::Float(value);
    assert_eq!(
        throttle.offer("a", float(1.), start),
        Offer::SendNow(float(1.))
    );
    // Inside the interval: deferred, one flush scheduled for the remainder.
    let second = throttle.offer("a", float(2.), start + Duration::from_millis(10));
    assert_eq!(
        second,
        Offer::Deferred {
            flush_after: Some(Duration::from_millis(20))
        }
    );
    // A third edit only replaces the pending value.
    assert_eq!(
        throttle.offer("a", float(3.), start + Duration::from_millis(15)),
        Offer::Deferred { flush_after: None }
    );
    // Other properties are independent.
    assert_eq!(
        throttle.offer("b", float(9.), start + Duration::from_millis(16)),
        Offer::SendNow(float(9.))
    );
    assert_eq!(
        throttle.flush("a", start + Duration::from_millis(30)),
        Some(float(3.))
    );
    assert_eq!(throttle.flush("a", start + Duration::from_millis(31)), None);
    // After a quiet interval the next edit goes straight out again.
    assert_eq!(
        throttle.offer("a", float(4.), start + Duration::from_millis(80)),
        Offer::SendNow(float(4.))
    );
}

#[test]
fn throttle_cancel_drops_the_pending_value() {
    let mut throttle = EditThrottle::new(Duration::from_millis(30));
    let start = Instant::now();
    let _ = throttle.offer("a", DebugValue::Float(1.), start);
    let _ = throttle.offer("a", DebugValue::Float(2.), start + Duration::from_millis(5));
    throttle.cancel("a", start + Duration::from_millis(6));
    assert_eq!(throttle.flush("a", start + Duration::from_millis(40)), None);
}
