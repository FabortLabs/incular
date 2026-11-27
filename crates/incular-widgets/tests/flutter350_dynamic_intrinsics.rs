//! The same mounted intrinsic dependency chain receives a leaf size update.

use incular_config::{Alignment, Constraints};
use incular_core::Size;
use incular_widgets::{
    Align, IntrinsicHeight, IntrinsicWidth, SizedBox, Widget,
    internal::{ElementId, Key, WidgetTree},
};

fn frame() -> Constraints {
    Constraints::at_most(1000., 1000.)
}

fn leaf(dimension: f32) -> Widget {
    Widget::from(SizedBox::square(dimension)).with_key("fixed-size leaf")
}

fn element(tree: &WidgetTree, key: &str) -> ElementId {
    tree.element_with_key(&Key::from(key)).unwrap()
}

fn fixture() -> WidgetTree {
    // Like the original's sized-by-parent proxy, an unfactored Align can
    // fill bounded space while forwarding its child's natural dimensions.
    let parent = Widget::from(Align::new(Alignment::CENTER, leaf(100.)))
        .with_key("fills bounds but delegates intrinsics");
    let root =
        Widget::from(IntrinsicWidth::new(IntrinsicHeight::new(parent))).with_key("intrinsic root");
    let mut tree = WidgetTree::new();
    tree.mount(root).unwrap();
    tree.layout(frame()).unwrap();
    tree
}

#[test]
fn changing_only_the_leaf_resizes_an_existing_intrinsic_root() {
    let mut tree = fixture();
    let root = element(&tree, "intrinsic root");
    let child = element(&tree, "fixed-size leaf");
    for dimension in [100., 200.] {
        if dimension == 200. {
            tree.update(child, leaf(dimension)).unwrap();
            tree.layout(frame()).unwrap();
        }
        assert_eq!(element(&tree, "intrinsic root"), root);
        assert_eq!(element(&tree, "fixed-size leaf"), child);
        for id in [root, child] {
            assert_eq!(
                tree.element_bounds(id).unwrap().size,
                Size::new(dimension, dimension)
            );
        }
    }
}

#[test]
fn intermediate_parent_refreshes_all_four_intrinsics_after_leaf_growth() {
    let mut tree = fixture();
    let parent = element(&tree, "fills bounds but delegates intrinsics");
    let render = tree.render_id(parent).unwrap();
    let child = element(&tree, "fixed-size leaf");
    for expected in [100., 200.] {
        if expected == 200. {
            tree.update(child, leaf(expected)).unwrap();
            tree.layout(frame()).unwrap();
        }
        assert_eq!(tree.render_id(parent), Some(render));
        assert_eq!(
            tree.min_intrinsic_width(render, f32::INFINITY),
            Some(expected)
        );
        assert_eq!(
            tree.max_intrinsic_width(render, f32::INFINITY),
            Some(expected)
        );
        assert_eq!(
            tree.min_intrinsic_height(render, f32::INFINITY),
            Some(expected)
        );
        assert_eq!(
            tree.max_intrinsic_height(render, f32::INFINITY),
            Some(expected)
        );
    }
}
