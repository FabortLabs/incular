//! Mounted layout controls for the platform-neutral Flutter toolbar delegate.

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_widgets::{
    CustomSingleChildLayout, Directionality, TextDirection, TextSelectionToolbarLayoutDelegate,
    Widget,
    internal::{ElementId, Key, WidgetTree},
};

const OVERLAY: Size = Size {
    width: 800.,
    height: 600.,
};
const TOOLBAR: Size = Size {
    width: 200.,
    height: 43.,
};
const BELOW: Offset = Offset::new(50., 500.);

fn scene(above: Offset, below: Offset, fits_above: Option<bool>) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        CustomSingleChildLayout::new(
            TextSelectionToolbarLayoutDelegate {
                anchor_above: above,
                anchor_below: below,
                fits_above,
            },
            Widget::box_(TOOLBAR, Color::rgba(255, 0, 0, 255)).with_key("selection toolbar"),
        ),
    )
    .into()
}

fn pump(tree: &mut WidgetTree) -> (ElementId, Rect) {
    tree.layout(Constraints::tight(OVERLAY)).unwrap();
    let child = tree
        .element_with_key(&Key::from("selection toolbar"))
        .unwrap();
    (child, tree.element_bounds(child).unwrap())
}

#[test]
fn retained_toolbar_flips_at_the_exact_fit_threshold() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(Offset::new(50., 0.), BELOW, None))
        .unwrap();
    let (toolbar, initial_bounds) = pump(&mut tree);
    assert_eq!(
        initial_bounds,
        Rect::from_origin_size(Offset::new(0., 500.), TOOLBAR)
    );
    for (above_y, expected_y) in [(42., 500.), (43., 0.)] {
        tree.update(root, scene(Offset::new(50., above_y), BELOW, None))
            .unwrap();
        assert_eq!(
            pump(&mut tree),
            (
                toolbar,
                Rect::from_origin_size(Offset::new(0., expected_y), TOOLBAR)
            )
        );
    }
}

#[test]
fn horizontal_placement_uses_the_chosen_anchor_and_keeps_the_toolbar_inside() {
    let mut tree = WidgetTree::new();
    let above = Offset::new(775., 80.);
    let below = Offset::new(400., 500.);
    let root = tree.mount(scene(above, below, None)).unwrap();
    let (toolbar, bounds) = pump(&mut tree);
    assert_eq!(
        bounds,
        Rect::from_origin_size(Offset::new(600., 37.), TOOLBAR)
    );

    tree.update(root, scene(above, below, Some(false))).unwrap();
    assert_eq!(
        pump(&mut tree),
        (
            toolbar,
            Rect::from_origin_size(Offset::new(300., 500.), TOOLBAR)
        )
    );

    tree.update(root, scene(Offset::new(20., 20.), below, Some(true)))
        .unwrap();
    assert_eq!(
        pump(&mut tree),
        (toolbar, Rect::from_origin_size(Offset::ZERO, TOOLBAR))
    );
}
