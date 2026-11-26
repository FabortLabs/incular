//! Float and pin real persistent headers as the cross-axis group scrolls.
use std::{cell::Cell, rc::Rc, time::Instant};

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Offset, PointerDeviceKind, PointerPhase, Rect, Size};
use incular_gestures::PointerEvent;
use incular_scroll::{ScrollController, SliverConstraints, SliverGeometry};
use incular_widgets::{
    CustomScrollView, Directionality, GlobalKey, SizedBox, Sliver, SliverCrossAxisGroup,
    SliverHeaderScrollBehavior, SliverPersistentHeader, SliverToBoxAdapter, Widget,
    internal::{RenderSliver, SliverChildId, SliverLayout, SliverScrollDependency, WidgetTree},
};

struct ObservedSliver<S>(S, Rc<Cell<SliverGeometry>>);
struct ObservedRenderSliver(Box<dyn RenderSliver>, Rc<Cell<SliverGeometry>>);

impl<S: Sliver> Sliver for ObservedSliver<S> {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.0.build(controller)
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(ObservedRenderSliver(
            self.0.create_render_sliver(controller, axis, reverse),
            self.1.clone(),
        ))
    }
}

impl RenderSliver for ObservedRenderSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let layout = self.0.perform_layout(constraints);
        self.1.set(layout.geometry);
        layout
    }

    fn set_child_extent(&mut self, child: SliverChildId, main_extent: f32) -> bool {
        self.0.set_child_extent(child, main_extent)
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.0.scroll_layout_dependency()
    }
}

const VIEW: Size = Size {
    width: 300.,
    height: 600.,
};

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(VIEW)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
}

fn mounted_header(
    behavior: SliverHeaderScrollBehavior,
    min: f32,
) -> (
    WidgetTree,
    ScrollController,
    Rc<Cell<SliverGeometry>>,
    GlobalKey,
) {
    let controller = ScrollController::new();
    let key = GlobalKey::new();
    let geometry = Rc::new(Cell::new(SliverGeometry::ZERO));
    let header = SliverPersistentHeader::with_delegate(min, 60., move |_, _| {
        Widget::from(SizedBox::new().height(60.)).with_key(key)
    })
    .scroll_behavior(behavior);
    let group = SliverCrossAxisGroup::new(vec![
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(600.))) as Box<dyn Sliver>,
        Box::new(ObservedSliver(header, geometry.clone())),
    ]);
    let viewport = CustomScrollView::new(vec![
        Box::new(group) as Box<dyn Sliver>,
        Box::new(SliverToBoxAdapter::new(SizedBox::new().height(2400.))),
    ])
    .controller(controller.clone());
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, viewport).into())
        .unwrap();
    layout(&mut tree);
    (tree, controller, geometry, key)
}

fn send(tree: &mut WidgetTree, time: Instant, phase: PointerPhase, y: f32) {
    tree.dispatch_pointer_gesture_in_window(
        0,
        1,
        PointerDeviceKind::Touch,
        PointerEvent {
            pointer: 1,
            position: Offset::new(150., y),
            phase,
            time,
        },
    );
    layout(tree);
}

#[test]
fn floating_header_reveals_and_pinned_floating_header_recedes_on_real_touch() {
    use SliverHeaderScrollBehavior::{Floating, FloatingPinned};
    for (behavior, min, positions) in [
        (Floating, 60., vec![(40., 40., -20., 60.)]),
        (
            Floating,
            40.,
            vec![(30., 30., -10., 40.), (50., 50., 0., 50.)],
        ),
        (
            FloatingPinned,
            40.,
            vec![(30., 40., -10., 40.), (50., 50., 0., 50.)],
        ),
    ] {
        let (mut tree, scroll, geometry, key) = mounted_header(behavior, min);
        assert!(scroll.jump_to(600.));
        layout(&mut tree);
        let time = Instant::now();
        send(&mut tree, time, PointerPhase::Down, 300.);
        for (dy, paint, y, height) in positions {
            send(&mut tree, time, PointerPhase::Move, 300. + dy);
            assert_eq!(scroll.offset(), 600. - dy, "{behavior:?}, min={min}");
            assert_eq!(
                geometry.get().paint_extent,
                paint,
                "{behavior:?}, min={min}"
            );
            let node = tree.element_with_key(&key.into()).unwrap();
            assert_eq!(
                tree.element_bounds(node),
                Some(Rect::from_origin_size(
                    Offset::new(150., y),
                    Size::new(150., height)
                )),
                "{behavior:?}, min={min}"
            );
        }
    }
}

#[test]
fn jumping_back_does_not_reveal_the_scrolling_or_floating_header() {
    for behavior in [
        SliverHeaderScrollBehavior::Scroll,
        SliverHeaderScrollBehavior::Floating,
    ] {
        let (mut tree, scroll, geometry, key) = mounted_header(behavior, 30.);
        assert!(scroll.jump_to(600.));
        layout(&mut tree);
        assert!(scroll.jump_to(570.));
        layout(&mut tree);
        assert_eq!(scroll.offset(), 570.);
        assert_eq!(geometry.get().paint_extent, 0.);
        let node = tree.element_with_key(&key.into()).unwrap();
        assert!(
            !tree
                .element_bounds(node)
                .unwrap()
                .intersects(Rect::from_origin_size(Offset::ZERO, VIEW))
        );
    }
}
