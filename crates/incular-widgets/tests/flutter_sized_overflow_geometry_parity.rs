//! Flutter 3.50.0 SizedOverflowBox alignment and zero-area geometry counterparts.

use incular_config::{Alignment, AlignmentDirectional, Constraints};
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    Center, Directionality, Placeholder, SizedBox, SizedOverflowBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

const OVERFLOW: u64 = 1;
const CHILD: u64 = 2;

fn inspect(
    direction: TextDirection,
    alignment: Alignment,
    viewport: Size,
    child: Widget,
    child_key: Option<u64>,
) -> (Rect, Option<Rect>) {
    let overflow = SizedOverflowBox::new(Size::new(100., 100.), child).alignment(alignment);
    let scene = Directionality::new(
        direction,
        Center::new(Widget::from(overflow).with_key(OVERFLOW)),
    );
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).expect("mount");
    tree.layout(Constraints::tight(viewport)).expect("layout");
    let bounds = |key| {
        let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
        tree.element_bounds(element).expect("fixture bounds")
    };
    (bounds(OVERFLOW), child_key.map(bounds))
}

fn inspect_alignment(direction: TextDirection, alignment: Alignment) -> (Rect, Rect) {
    let (outer, child) = inspect(
        direction,
        alignment,
        Size::new(800., 600.),
        Widget::from(SizedBox::square(50.)).with_key(CHILD),
        Some(CHILD),
    );
    (outer, child.expect("child recorded"))
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

// widgets/overflow_box_test.dart:115, "SizedOverflowBox alignment".
#[test]
fn absolute_top_right_aligns_natural_size_child_even_in_rtl() {
    assert_eq!(
        inspect_alignment(TextDirection::Rtl, Alignment::TOP_RIGHT),
        (rect(350., 250., 100., 100.), rect(400., 250., 50., 50.))
    );
}

// widgets/overflow_box_test.dart:142, "SizedOverflowBox alignment (direction-sensitive)".
// The Incular widget only accepts Alignment, so this covers geometry with
// the same resolved alignment, not inherited direction-sensitive API parity.
#[test]
fn resolved_rtl_bottom_start_places_natural_size_child_at_bottom_right() {
    let rtl = TextDirection::Rtl;
    assert_eq!(
        inspect_alignment(rtl, AlignmentDirectional::BOTTOM_START.resolve(rtl)),
        (rect(350., 250., 100., 100.), rect(400., 300., 50., 50.))
    );
}

// widgets/overflow_box_test.dart:169, "SizedOverflowBox does not crash at zero area".
#[test]
fn sized_overflow_box_constrains_its_preferred_size_to_zero_area() {
    let (outer, _) = inspect(
        TextDirection::Ltr,
        Alignment::CENTER,
        Size::ZERO,
        Placeholder::new().into(),
        None,
    );
    assert_eq!(outer, rect(0., 0., 0., 0.));
}

// Supplemental pinned Flutter probe: shifted_box.dart RenderSizedOverflowBox
// must pass its incoming constraints unchanged to the child.
#[test]
fn sized_overflow_box_keeps_outer_constraints_for_its_larger_child() {
    let (outer, child) = inspect(
        TextDirection::Ltr,
        Alignment::CENTER,
        Size::new(800., 600.),
        Widget::from(SizedBox::new().width(200.).height(150.)).with_key(CHILD),
        Some(CHILD),
    );
    assert_eq!(outer, rect(350., 250., 100., 100.));
    assert_eq!(child, Some(rect(300., 225., 200., 150.)));
}
