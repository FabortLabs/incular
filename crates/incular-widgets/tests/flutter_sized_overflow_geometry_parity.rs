//! Flutter 3.50.0 SizedOverflowBox alignment and zero-area geometry counterparts.

use incular_config::{Alignment, AlignmentDirectional, Constraints};
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    AlignmentGeometry, Center, Directionality, Placeholder, SizedBox, SizedOverflowBox,
    TextDirection, Widget,
    internal::{Key, WidgetTree},
};

const OVERFLOW: u64 = 1;
const CHILD: u64 = 2;

fn inspect(
    direction: TextDirection,
    alignment: impl Into<AlignmentGeometry>,
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

fn inspect_alignment(
    direction: TextDirection,
    alignment: impl Into<AlignmentGeometry>,
) -> (Rect, Rect) {
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
#[test]
fn inherited_rtl_bottom_start_places_natural_size_child_at_bottom_right() {
    assert_eq!(
        inspect_alignment(TextDirection::Rtl, AlignmentDirectional::BOTTOM_START),
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

#[test]
fn inherited_direction_repositions_the_same_sized_overflow_child() {
    let build = |direction| {
        let child = Widget::from(SizedBox::square(50.)).with_key(CHILD);
        let overflow = SizedOverflowBox::new(Size::new(100., 100.), child)
            .alignment(AlignmentDirectional::BOTTOM_START);
        Directionality::new(
            direction,
            Center::new(Widget::from(overflow).with_key(OVERFLOW)),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(TextDirection::Rtl)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.layout(constraints).unwrap();
    let overflow = tree.element_with_key(&Key::from(OVERFLOW)).unwrap();
    let child = tree.element_with_key(&Key::from(CHILD)).unwrap();
    let overflow_render = tree.render_id(overflow).unwrap();
    for (direction, child_x) in [
        (TextDirection::Rtl, 400.),
        (TextDirection::Ltr, 350.),
        (TextDirection::Rtl, 400.),
    ] {
        tree.update(root, build(direction)).unwrap();
        tree.layout(constraints).unwrap();
        assert_eq!(tree.element_with_key(&Key::from(OVERFLOW)), Some(overflow));
        assert_eq!(tree.render_id(overflow), Some(overflow_render));
        assert_eq!(tree.element_with_key(&Key::from(CHILD)), Some(child));
        assert_eq!(
            tree.element_bounds(child),
            Some(rect(child_x, 300., 50., 50.))
        );
    }
}
