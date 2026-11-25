//! Inherited horizontal direction on Flutter's built-in box scroll views.

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Offset, Rect, Size, time::Instant};
use incular_scroll::ScrollController;
use incular_widgets::{
    Directionality, GridView, ListView, PageController, PageView, SizedBox, SliverGridDelegate,
    Text, Widget,
    internal::{Key, WidgetTree},
};

#[derive(Clone, Copy, Debug)]
enum ViewKind {
    List,
    Grid,
    Pages,
}

fn compare(kind: ViewKind, reverse: bool) {
    let first = Key::from("first");
    let children = ["first", "second", "third"].map(|label| {
        let text = Widget::from(Text::new(label)).with_key(Key::from(label));
        Widget::from(SizedBox::new().width(250.).child(text))
    });
    let controller = ScrollController::new();
    let view: Widget = match kind {
        ViewKind::List => ListView::new(children)
            .scroll_direction(Axis::Horizontal)
            .item_extent(250.)
            .reverse(reverse)
            .controller(controller.clone())
            .into(),
        ViewKind::Grid => GridView::new(
            SliverGridDelegate::fixed_cross_axis_count(1).main_axis_extent(250.),
            children,
        )
        .scroll_direction(Axis::Horizontal)
        .reverse(reverse)
        .controller(controller.clone())
        .into(),
        ViewKind::Pages => PageView::new(children)
            .viewport_fraction(0.8)
            .pad_ends(false)
            .page_snapping(false)
            .reverse(reverse)
            .controller(PageController::from(controller.clone()).with_viewport_fraction(0.8))
            .into(),
    };
    let make = |direction| Directionality::new(direction, view.clone()).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(TextDirection::Ltr)).unwrap();
    let constraints = Constraints::tight(Size::new(300., 100.));
    tree.layout(constraints).unwrap();
    let child = tree.element_with_key(&first).unwrap();
    let width = if matches!(kind, ViewKind::Pages) {
        240.
    } else {
        250.
    };
    for direction in [TextDirection::Ltr, TextDirection::Rtl, TextDirection::Ltr] {
        tree.update(root, make(direction)).unwrap();
        tree.layout(constraints).unwrap();
        assert_eq!(tree.element_with_key(&first), Some(child));
        let leftward = reverse ^ (direction == TextDirection::Rtl);
        for offset in [0., 30.] {
            controller.jump_to(offset);
            tree.layout(constraints).unwrap();
            tree.update_compositor(Instant::now()).unwrap();
            let x = if leftward {
                300. - width + offset
            } else {
                -offset
            };
            assert_eq!(
                tree.element_bounds(child).unwrap(),
                Rect::from_origin_size(Offset::new(x, 0.), Size::new(width, 100.)),
                "{kind:?} reverse={reverse} {direction:?} offset={offset}"
            );
            assert_eq!(
                tree.hit_test(Offset::new(x + width / 2., 50.))
                    .and_then(|id| tree.element_for_render(id)),
                Some(child),
            );
        }
    }
}

#[test]
fn list_view_ltr_rtl_and_explicit_reverse() {
    compare(ViewKind::List, false);
    compare(ViewKind::List, true);
}

#[test]
fn grid_view_ltr_rtl_and_explicit_reverse() {
    compare(ViewKind::Grid, false);
    compare(ViewKind::Grid, true);
}

#[test]
fn page_view_ltr_rtl_and_explicit_reverse() {
    compare(ViewKind::Pages, false);
    compare(ViewKind::Pages, true);
}
