//! Selected Flutter 3.50.0 alignment geometry contracts.
use incular_config::{Alignment, AlignmentDirectional as Directional, TextDirection};
use incular_core::{Offset, Size};

#[test]
fn directional_alignment_resolves_in_ltr_and_rtl() {
    for (start, y) in [(0.25, 0.3), (-0.25, 0.3), (1.25, 0.3), (0.5, -0.3)] {
        let directional = Directional::new(start, y);
        assert_eq!(
            directional.resolve(TextDirection::Ltr),
            Alignment::new(start, y)
        );
        assert_eq!(
            directional.resolve(TextDirection::Rtl),
            Alignment::new(-start, y)
        );
    }
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        assert_eq!(Directional::CENTER.resolve(direction), Alignment::CENTER);
    }
    assert_eq!(
        Directional::BOTTOM_END.resolve(TextDirection::Ltr),
        Alignment::BOTTOM_RIGHT
    );
    assert_eq!(
        Directional::BOTTOM_END.resolve(TextDirection::Rtl),
        Alignment::BOTTOM_LEFT
    );
    assert_eq!(
        Directional::new(1.0, 2.0),
        Directional { start: 1.0, y: 2.0 }
    );
    assert_ne!(Directional::new(1.0, 2.0), Directional::new(2.0, 1.0));
    assert_eq!(
        Directional::CENTER_START.resolve(TextDirection::Ltr),
        Directional::CENTER_END.resolve(TextDirection::Rtl)
    );
    assert_ne!(
        Directional::CENTER_START.resolve(TextDirection::Ltr),
        Directional::CENTER_END.resolve(TextDirection::Ltr)
    );
    assert_ne!(
        Directional::CENTER_END.resolve(TextDirection::Ltr),
        Directional::CENTER_END.resolve(TextDirection::Rtl)
    );
}

#[test]
fn alignment_constructors_retain_values_outside_normalized_range() {
    assert_eq!(Alignment::new(4.0, 5.0), Alignment { x: 4.0, y: 5.0 });
    assert_eq!(
        Directional::new(4.0, 5.0),
        Directional { start: 4.0, y: 5.0 }
    );
}

#[test]
fn alignment_static_values_cover_absolute_and_directional_positions() {
    assert_eq!(
        [
            Alignment::TOP_LEFT,
            Alignment::TOP_CENTER,
            Alignment::TOP_RIGHT,
            Alignment::CENTER_LEFT,
            Alignment::CENTER,
            Alignment::CENTER_RIGHT,
            Alignment::BOTTOM_LEFT,
            Alignment::BOTTOM_CENTER,
            Alignment::BOTTOM_RIGHT,
        ]
        .map(|a| (a.x, a.y)),
        [
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (0.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 1.0)
        ],
    );
    assert_eq!(
        [
            Directional::TOP_START,
            Directional::TOP_END,
            Directional::CENTER_START,
            Directional::CENTER_END,
            Directional::BOTTOM_START,
            Directional::BOTTOM_END,
        ]
        .map(|a| (a.start, a.y)),
        [
            (-1.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (1.0, 1.0)
        ],
    );
}

#[test]
fn alignment_along_size_uses_the_top_left_origin() {
    let size = Size::new(100.0, 60.0);
    let offsets = [
        Alignment::TOP_LEFT,
        Alignment::CENTER,
        Alignment::BOTTOM_RIGHT,
        Alignment::new(1.5, -2.0),
    ]
    .map(|a| a.along_size(size));
    assert_eq!(
        offsets,
        [
            Offset::ZERO,
            Offset::new(50.0, 30.0),
            Offset::new(100.0, 60.0),
            Offset::new(125.0, -30.0)
        ]
    );
}
