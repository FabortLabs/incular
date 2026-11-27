use incular_core::{Offset, Rect, Size, TextDirection};
use incular_rendering::{SelectionResult, SelectionUtils};

fn rect() -> Rect {
    Rect::from_origin_size(Offset::new(100., 100.), Size::new(200., 500.))
}

const POINTS: [(Offset, SelectionResult); 9] = [
    (Offset::new(50., 50.), SelectionResult::Previous),
    (Offset::new(50., 200.), SelectionResult::Previous),
    (Offset::new(50., 700.), SelectionResult::Next),
    (Offset::new(200., 50.), SelectionResult::Previous),
    (Offset::new(350., 50.), SelectionResult::Previous),
    (Offset::new(350., 200.), SelectionResult::Next),
    (Offset::new(350., 700.), SelectionResult::Next),
    (Offset::new(200., 700.), SelectionResult::Next),
    (Offset::new(150., 300.), SelectionResult::End),
];

#[test]
fn rectangle_selection_classifies_all_eight_outside_regions_and_the_center() {
    for (point, expected) in POINTS {
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(rect(), point),
            expected,
            "{point:?}"
        );
    }
}

#[test]
fn drag_offsets_snap_to_directional_corners_only_outside_the_rectangle() {
    use SelectionResult::{End, Next, Pending, Previous};
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let (first, last) = if direction == TextDirection::Ltr {
            (Offset::new(100., 100.), Offset::new(300., 600.))
        } else {
            (Offset::new(300., 100.), Offset::new(100., 600.))
        };
        for (point, placement) in POINTS {
            let expected = match placement {
                Previous => first,
                Next => last,
                End => point,
                Pending => unreachable!("fixture classification"),
            };
            assert_eq!(
                SelectionUtils::adjust_drag_offset(rect(), point, direction),
                expected,
                "{point:?} {direction:?}"
            );
        }
    }
}

#[test]
fn flutter_selection_keeps_its_distinct_policies_at_exact_edges() {
    use SelectionResult::{End, Next, Previous};
    for (point, classification, adjusted) in [
        (Offset::new(100., 100.), End, Offset::new(100., 100.)),
        (Offset::new(300., 100.), Next, Offset::new(100., 100.)),
        (Offset::new(300., 300.), Next, Offset::new(300., 600.)),
        (Offset::new(100., 600.), Previous, Offset::new(100., 100.)),
        (Offset::new(200., 600.), Previous, Offset::new(300., 600.)),
    ] {
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(rect(), point),
            classification
        );
        assert_eq!(
            SelectionUtils::adjust_drag_offset(rect(), point, TextDirection::Ltr),
            adjusted
        );
    }
}
