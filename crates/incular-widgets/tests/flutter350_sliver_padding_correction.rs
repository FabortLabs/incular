//! Retained anchor correction regression paired with a pinned Flutter probe.

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Offset, Rect, Size};
use incular_scroll::ScrollController;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{CustomScrollView, Sliver, SliverPadding, Widget};

fn check_rewind(new_height: f32, top: f32, verify_later_visible_resize: bool) {
    use incular_widgets::SliverList;

    let controller = ScrollController::new();
    let scene = |leading_height: f32| {
        let list = SliverList::builder(10, move |index| {
            let height = if index == 0 { leading_height } else { 200. };
            Widget::box_(Size::new(800., height), Color::WHITE).with_key(Key::from(index as u64))
        });
        let slivers: Vec<Box<dyn Sliver>> =
            vec![Box::new(SliverPadding::new(EdgeInsets::ZERO, list))];
        CustomScrollView::new(slivers)
            .cache_extent(0.)
            .controller(controller.clone())
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(200.)).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    tree.layout(view).unwrap();
    assert!(controller.jump_to(300.));
    tree.layout(view).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();

    let row = tree.element_with_key(&Key::from(2_u64)).unwrap();
    let wanted = Rect::from_origin_size(Offset::new(0., 100.), Size::new(800., 200.));
    assert_eq!(tree.element_bounds(row).unwrap(), wanted);

    tree.update(root, scene(new_height)).unwrap();
    tree.layout(view).unwrap();
    assert!(controller.jump_to(0.));
    tree.layout(view).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    let first = tree.element_with_key(&Key::from(0_u64)).unwrap();
    let wanted = Rect::from_origin_size(Offset::new(0., top), Size::new(800., new_height));
    assert_eq!(tree.element_bounds(first).unwrap(), wanted);
    assert_eq!(controller.offset(), -top);

    if verify_later_visible_resize {
        // Once index zero is actually retained, growing it is not a
        // backward insertion. A prior unchanged rewind must not leave
        // behind a correction that fires on this ordinary rebuild.
        tree.update(root, scene(400.)).unwrap();
        tree.layout(view).unwrap();
        let first = tree.element_with_key(&Key::from(0_u64)).unwrap();
        assert_eq!(controller.offset(), 0.);
        assert_eq!(
            tree.element_bounds(first).unwrap(),
            Rect::from_origin_size(Offset::ZERO, Size::new(800., 400.)),
        );
    }
}

#[test]
fn padding_preserves_list_offset_correction_when_a_leading_item_grows() {
    check_rewind(400., -200., false);
}

#[test]
fn unchanged_leading_item_rewinds_to_zero_and_a_later_resize_stays_there() {
    check_rewind(200., 0., true);
}

#[test]
fn a_smaller_leading_item_does_not_create_a_positive_anchor_correction() {
    check_rewind(100., 0., false);
}
