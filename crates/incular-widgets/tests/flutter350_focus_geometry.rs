use std::f32::consts::PI;

use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, Padding, Transform, Widget, internal::WidgetTree,
};

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} vs {expected}");
}

fn bounds(node: &FocusNode, origin: Offset, width: f32, height: f32) {
    let actual = node.rect();
    close(actual.origin.x, origin.x);
    close(actual.origin.y, origin.y);
    close(actual.size.width, width);
    close(actual.size.height, height);
    assert_eq!(node.offset(), actual.origin);
    assert_eq!(node.size(), actual.size);
}

#[test]
fn focus_rect_follows_transformed_corners_instead_of_sorting_them() {
    for hosted in [false, true] {
        let mut tree = WidgetTree::new();
        if hosted {
            tree.enable_hosted_focus();
        }
        let first = FocusNode::new();
        let second = FocusNode::new();
        let child = |node: &FocusNode| {
            Focus::new(Widget::box_(Size::new(200., 100.), Color::WHITE)).node(node.clone())
        };
        tree.mount(
            Directionality::new(
                TextDirection::Ltr,
                Padding::new(
                    EdgeInsets::all(8.),
                    Column::new([
                        Widget::from(child(&first)),
                        Transform::translation(
                            Offset::new(10., 20.),
                            Transform::scale(0.33, Transform::rotation(PI, child(&second))),
                        )
                        .into(),
                    ]),
                ),
            )
            .into(),
        )
        .unwrap();
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();

        // Geometry must be available before either focus or a traversal scan.
        bounds(&first, Offset::new(300., 8.), 200., 100.);
        bounds(&second, Offset::new(443., 194.5), -66., -33.);
        assert_eq!(tree.focusable_elements().len(), 2);
        bounds(&second, Offset::new(443., 194.5), -66., -33.);
    }
}

#[test]
fn paint_only_transforms_refresh_geometry_even_for_a_nontraversable_focus_node() {
    let node = FocusNode::new();
    node.set_skip_traversal(true);
    let scene = |scale: f32| {
        Widget::from(Transform::scale(
            scale,
            Focus::new(Widget::box_(Size::new(100., 60.), Color::WHITE)).node(node.clone()),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(1.)).unwrap();
    let constraints = Constraints::loose(Size::new(240., 120.));
    tree.layout(constraints).unwrap();
    assert!(tree.focusable_elements().is_empty());
    assert_eq!(
        node.bounds(),
        Rect::from_origin_size(Offset::ZERO, Size::new(100., 60.))
    );

    tree.update(root, scene(0.5)).unwrap();
    tree.layout(constraints).unwrap();
    bounds(&node, Offset::new(25., 15.), 50., 30.);
    assert!(tree.focusable_elements().is_empty());
}
