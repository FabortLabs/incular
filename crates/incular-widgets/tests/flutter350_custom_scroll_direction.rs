//! Flutter horizontal ScrollView axes, padding edges, retained geometry, and hits.

use std::{cell::Cell, rc::Rc};

use incular_config::{Axis, Constraints, EdgeInsetsDirectional, TextDirection};
use incular_core::{Offset, Rect, Size, time::Instant};
use incular_widgets::{
    CustomScrollView, Directionality, SizedBox, Sliver, SliverPadding, SliverToBoxAdapter, Text,
    Viewport, Widget,
    internal::{Key, WidgetTree},
};

fn scene(
    controller: &incular_scroll::ScrollController,
    reverse: bool,
    raw: bool,
    padding: bool,
) -> Widget {
    let row = |label| {
        SliverToBoxAdapter::new(
            SizedBox::new()
                .width(250.)
                .height(100.)
                .child(Widget::from(Text::new(label)).with_key(Key::from(label))),
        )
    };
    let first: Box<dyn Sliver> = if padding {
        Box::new(SliverPadding::new(
            EdgeInsetsDirectional::only(20., 0., 5., 0.),
            row("first"),
        ))
    } else {
        Box::new(row("first"))
    };
    let slivers = [first, Box::new(row("second"))];
    if raw {
        Viewport::new(slivers)
            .axis_direction(Axis::Horizontal)
            .reverse(reverse)
            .controller(controller.clone())
            .into()
    } else {
        CustomScrollView::new(slivers)
            .scroll_direction(Axis::Horizontal)
            .reverse(reverse)
            .controller(controller.clone())
            .into()
    }
}

fn compare(raw: bool, reverse: bool, padding: bool, expected: [f32; 3]) {
    use TextDirection::{Ltr, Rtl};

    let controller = incular_scroll::ScrollController::new();
    let view = scene(&controller, reverse, raw, padding);
    let build = |direction| Directionality::new(direction, view.clone()).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(Ltr)).unwrap();
    let constraints = Constraints::tight(Size::new(300., 100.));
    tree.layout(constraints).unwrap();
    let key = Key::from("first");
    let first = tree.element_with_key(&key).unwrap();
    for (direction, x) in [Ltr, Rtl, Ltr].into_iter().zip(expected) {
        tree.update(root, build(direction)).unwrap();
        tree.layout(constraints).unwrap();
        tree.update_compositor(Instant::now()).unwrap();
        assert_eq!(tree.element_with_key(&key), Some(first));
        assert_eq!(
            tree.element_bounds(first).unwrap(),
            Rect::from_origin_size(Offset::new(x, 0.), Size::new(250., 100.)),
            "{direction:?} raw={raw} reverse={reverse} padding={padding}",
        );
        assert_eq!(controller.offset(), 0.);
        for (point, hits) in [
            (x + 1., true),
            (x + 249., true),
            (1., x == 0.),
            (299., x == 50.),
        ] {
            let actual = tree
                .hit_test(Offset::new(point, 50.))
                .and_then(|id| tree.element_for_render(id));
            assert_eq!(actual == Some(first), hits, "x={point} child starts at {x}");
        }
        if !raw {
            assert!(controller.jump_to(30.));
            tree.layout(constraints).unwrap();
            tree.update_compositor(Instant::now()).unwrap();
            let leftward = reverse ^ (direction == Rtl);
            assert_eq!(
                tree.element_bounds(first).unwrap(),
                Rect::from_origin_size(
                    Offset::new(x + if leftward { 30. } else { -30. }, 0.),
                    Size::new(250., 100.),
                ),
            );
            assert!(controller.jump_to(0.));
            tree.layout(constraints).unwrap();
        }
    }
}

#[test]
fn horizontal_custom_scroll_view_uses_ambient_direction_then_reverse() {
    compare(false, false, false, [0., 50., 0.]);
    compare(false, true, false, [50., 0., 50.]);
}

#[test]
fn horizontal_directional_padding_is_not_applied_twice() {
    compare(false, false, true, [20., 30., 20.]);
    compare(false, true, true, [45., 5., 45.]);
    compare(true, false, true, [20., 5., 20.]);
    compare(true, true, true, [45., 30., 45.]);
}

#[test]
fn custom_sliver_receives_the_same_physical_direction_as_layout() {
    struct DirectionProbe(Rc<Cell<Option<bool>>>);
    impl Sliver for DirectionProbe {
        fn build(&self, _: &incular_scroll::ScrollController) -> Widget {
            SizedBox::shrink().into()
        }

        fn build_with_config(
            &self,
            controller: &incular_scroll::ScrollController,
            axis: Axis,
            reverse: bool,
        ) -> Widget {
            assert_eq!(axis, Axis::Horizontal);
            self.0.set(Some(reverse));
            self.build(controller)
        }
    }

    let observed = Rc::new(Cell::new(None));
    let probe: Box<dyn Sliver> = Box::new(DirectionProbe(observed.clone()));
    let scroll = CustomScrollView::new([probe]).scroll_direction(Axis::Horizontal);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Rtl, scroll).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(100., 20.)))
        .unwrap();
    assert_eq!(observed.get(), Some(true));
}
