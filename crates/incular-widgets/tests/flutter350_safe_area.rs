use incular_config::{Constraints, EdgeInsets, RuntimeEnvironment};
use incular_core::{Color, Offset, Rect, Size};
use incular_widgets::{
    Center, Column, Expanded, MediaQuery, SafeArea, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

const FRAME: Size = Size {
    width: 800.,
    height: 600.,
};

fn leaf() -> Widget {
    Widget::box_(Size::new(16., 16.), Color::WHITE).with_key(Key::from("placeholder"))
}

fn bounds(tree: &WidgetTree) -> Rect {
    tree.element_bounds(tree.element_with_key(&Key::from("placeholder")).unwrap())
        .unwrap()
}

fn mount(child: Widget, data: RuntimeEnvironment) -> Rect {
    let mut tree = WidgetTree::new();
    tree.mount(MediaQuery::new(data, child).into()).unwrap();
    tree.layout(Constraints::tight(FRAME)).unwrap();
    bounds(&tree)
}

fn corners(rect: Rect) -> (Offset, Offset) {
    (
        rect.origin,
        Offset::new(
            rect.origin.x + rect.size.width,
            rect.origin.y + rect.size.height,
        ),
    )
}

fn padding(safe_insets: EdgeInsets) -> RuntimeEnvironment {
    RuntimeEnvironment {
        safe_insets,
        ..RuntimeEnvironment::default()
    }
}

#[test]
fn basic_minimum_and_nested_safe_area_matches_exact_flutter_corners() {
    let ambient = padding(EdgeInsets::all(20.));
    for (child, raw) in [
        (
            SafeArea::new(leaf()).left(false).into(),
            (0., 20., 780., 580.),
        ),
        (
            SafeArea::new(leaf())
                .top(false)
                .minimum(EdgeInsets::only(0., 10., 20., 30.))
                .into(),
            (20., 10., 780., 570.),
        ),
        (
            SafeArea::new(SafeArea::new(leaf()).right(false))
                .top(false)
                .into(),
            (20., 20., 780., 580.),
        ),
    ] {
        assert_eq!(
            corners(mount(child, ambient.clone())),
            (Offset::new(raw.0, raw.1), Offset::new(raw.2, raw.3))
        );
    }
}

#[test]
fn nested_safe_area_updates_inherited_padding_without_replacing_child() {
    let child: Widget = SafeArea::new(SafeArea::new(leaf()).left(false).bottom(false))
        .bottom(false)
        .into();
    let scoped = |safe| MediaQuery::new(padding(safe), child.clone()).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(scoped(EdgeInsets::all(20.))).unwrap();
    tree.layout(Constraints::tight(FRAME)).unwrap();
    assert_eq!(
        corners(bounds(&tree)),
        (Offset::new(20., 20.), Offset::new(780., 600.))
    );
    let original = tree.element_with_key(&Key::from("placeholder"));
    tree.update(root, scoped(EdgeInsets::only(100., 30., 0., 40.)))
        .unwrap();
    tree.layout(Constraints::tight(FRAME)).unwrap();
    assert_eq!(
        corners(bounds(&tree)),
        (Offset::new(100., 30.), Offset::new(800., 600.))
    );
    assert_eq!(tree.element_with_key(&Key::from("placeholder")), original);
}

#[test]
fn maintained_safe_area_preserves_center_with_full_or_partial_keyboard_insets() {
    for fixed_size_parent in [false, true] {
        for full_occlusion in [false, true] {
            let body: Widget = Column::new([Widget::from(Expanded::new(leaf()))]).into();
            let body = if fixed_size_parent {
                Widget::from(SizedBox::expand().child(body))
            } else {
                body
            };
            let child: Widget = SafeArea::new(body)
                .maintain_bottom_view_padding(true)
                .into();
            let scoped = |top, bottom, keyboard| {
                MediaQuery::new(
                    RuntimeEnvironment {
                        safe_insets: EdgeInsets::only(0., top, 0., bottom),
                        view_padding: EdgeInsets::only(0., 0., 0., 20.),
                        view_insets: EdgeInsets::only(0., 0., 0., keyboard),
                        ..Default::default()
                    },
                    child.clone(),
                )
                .into()
            };
            let mut tree = WidgetTree::new();
            let top = if full_occlusion { 20. } else { 0. };
            let root = tree.mount(scoped(top, top, 0.)).unwrap();
            tree.layout(Constraints::tight(FRAME)).unwrap();
            let prior = bounds(&tree);
            tree.update(
                root,
                scoped(top, 0., if full_occlusion { 300. } else { 10. }),
            )
            .unwrap();
            tree.layout(Constraints::tight(FRAME)).unwrap();
            assert_eq!(bounds(&tree), prior);
            assert_eq!(prior.origin.y, top);
            assert_eq!(prior.size.height, 600. - top - 20.);
        }
    }
}

#[test]
fn maintained_bottom_uses_persistent_padding_and_can_be_consumed_by_ancestor() {
    let persistent = EdgeInsets::only(0., 0., 0., 20.);
    let child = SafeArea::new(leaf()).maintain_bottom_view_padding(true);
    assert_eq!(
        bounds_height(mount(
            child.clone().into(),
            RuntimeEnvironment {
                safe_insets: EdgeInsets::only(0., 0., 0., 35.),
                view_padding: persistent,
                ..Default::default()
            }
        )),
        580.
    );
    let nested: Widget = SafeArea::new(child)
        .maintain_bottom_view_padding(true)
        .into();
    for (current, expected_height) in [(20., 580.), (0., 560.)] {
        assert_eq!(
            bounds_height(mount(
                nested.clone(),
                RuntimeEnvironment {
                    safe_insets: EdgeInsets::only(0., 0., 0., current),
                    view_padding: persistent,
                    ..Default::default()
                }
            )),
            expected_height
        );
    }
}

fn bounds_height(rect: Rect) -> f32 {
    rect.size.height
}

#[test]
fn safe_area_at_zero_size_is_finite() {
    let child: Widget = Center::new(SizedBox::shrink().child(SafeArea::new(leaf()))).into();
    assert_eq!(mount(child, RuntimeEnvironment::default()).size, Size::ZERO);
}
