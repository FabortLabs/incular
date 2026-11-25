//! Exact layout and projected corners from Flutter fitted_box_test.dart:115.

use incular_config::{AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_widgets::{
    BoxFit, Center, Directionality, FittedBox, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

#[test]
fn fitted_box_moves_the_same_child_with_direction_alignment_size_and_fit() {
    let outer = Key::from("fitted alignment");
    let inner = Key::from("child alignment");
    let scene = |direction, alignment, size, fit| {
        let child = Widget::from(SizedBox::from_size(size)).with_key(inner.clone());
        let fitted = Widget::from(FittedBox::new(child).alignment(alignment).fit(fit))
            .with_key(outer.clone());
        Directionality::new(direction, Center::new(SizedBox::square(100.).child(fitted))).into()
    };
    let mut tree = WidgetTree::new();
    let small = Size::new(10., 10.);
    let large = Size::new(30., 10.);
    let end = AlignmentDirectional::BOTTOM_END;
    let center = AlignmentDirectional::CENTER;
    let root = tree
        .mount(scene(TextDirection::Rtl, end, small, BoxFit::ScaleDown))
        .unwrap();
    let mut retained = None;
    for (direction, align, size, fit, first, last) in [
        (
            TextDirection::Rtl,
            end,
            small,
            BoxFit::ScaleDown,
            Offset::new(0., 90.),
            Offset::new(10., 100.),
        ),
        (
            TextDirection::Ltr,
            end,
            small,
            BoxFit::ScaleDown,
            Offset::new(90., 90.),
            Offset::new(100., 100.),
        ),
        (
            TextDirection::Ltr,
            center,
            small,
            BoxFit::ScaleDown,
            Offset::new(45., 45.),
            Offset::new(55., 55.),
        ),
        (
            TextDirection::Ltr,
            center,
            large,
            BoxFit::ScaleDown,
            Offset::new(35., 45.),
            Offset::new(65., 55.),
        ),
        (
            TextDirection::Ltr,
            center,
            large,
            BoxFit::Fill,
            Offset::ZERO,
            Offset::new(100., 100.),
        ),
    ] {
        tree.update(root, scene(direction, align, size, fit))
            .unwrap();
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let outside = tree.element_with_key(&outer).unwrap();
        let inside = tree.element_with_key(&inner).unwrap();
        let outside_render = tree.render_id(outside).unwrap();
        assert_eq!(outside_render, *retained.get_or_insert(outside_render));
        assert_eq!(
            tree.render_size(outside_render),
            Some(Size::new(100., 100.))
        );
        assert_eq!(
            tree.render_size(tree.render_id(inside).unwrap()),
            Some(size)
        );
        let origin = tree.element_bounds(outside).unwrap().origin;
        let child = tree.element_bounds(inside).unwrap();
        let projected_start = child.origin - origin;
        let projected_end = projected_start + Offset::new(child.size.width, child.size.height);
        assert_eq!((projected_start, projected_end), (first, last));
    }
}
