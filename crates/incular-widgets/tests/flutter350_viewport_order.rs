//! Centered viewport paint callbacks and full translucent pointer route.

use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_config::{Axis, Constraints};
use incular_core::{Color, Offset, PointerDeviceKind, PointerPhase, Rect, Size};
use incular_rendering::{Canvas, DisplayList, PaintCommand};
use incular_scroll::{ScrollController, SliverConstraints, SliverGeometry};
use incular_widgets::{
    CustomPaint, CustomPainter, CustomScrollView, Directionality, HitTestBehavior, Listener,
    RawPointerEvent, SizedBox, Sliver, SliverPaintOrder, SliverToBoxAdapter, TextDirection, Widget,
    internal::{
        RenderSliver, SliverChildId, SliverChildLayout, SliverChildPlacement, SliverLayout,
        WidgetTree,
    },
};

type Trace = Rc<RefCell<Vec<u8>>>;

fn at_center(slivers: Vec<Box<dyn Sliver>>, order: Option<SliverPaintOrder>) -> WidgetTree {
    let mut viewport = CustomScrollView::new(slivers).center(2).anchor(0.5);
    if let Some(order) = order {
        viewport = viewport.paint_order(order);
    }
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, viewport).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    tree
}

struct PaintRecorder {
    id: u8,
    trace: Trace,
}

impl CustomPainter for PaintRecorder {
    fn paint(&self, size: Size) -> DisplayList {
        self.trace.borrow_mut().push(self.id);
        let mut canvas = Canvas::default();
        canvas.rect(
            Rect::from_origin_size(Offset::ZERO, size),
            Color::rgba(200, self.id, 63, 255),
        );
        canvas.finish()
    }

    fn should_repaint(&self, _old: &Self) -> bool {
        true
    }
}

#[test]
fn centered_slivers_paint_in_configured_callback_and_display_order() {
    for (order, expected) in [
        (None, [4, 3, 2, 1, 0]),
        (Some(SliverPaintOrder::LastIsTop), [0, 1, 2, 3, 4]),
    ] {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let slivers = (0..5)
            .map(|id| -> Box<dyn Sliver> {
                Box::new(SliverToBoxAdapter::new(CustomPaint::from_painter(
                    Size::new(800., 24.),
                    PaintRecorder {
                        id,
                        trace: trace.clone(),
                    },
                )))
            })
            .collect();
        let mut tree = at_center(slivers, order);
        let list = tree.try_paint().unwrap();
        assert_eq!(&*trace.borrow(), &expected, "{order:?} callbacks");
        let recorded: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|cmd| match cmd {
                PaintCommand::Rect { color, .. } if color.red == 200 && color.blue == 63 => {
                    Some(color.green)
                }
                _ => None,
            })
            .collect();
        assert_eq!(recorded, expected, "{order:?} composited pictures");
    }
}

#[derive(Clone)]
struct OverlappingSliver {
    id: u8,
    trace: Trace,
}

impl OverlappingSliver {
    fn listener(&self) -> Widget {
        let id = self.id;
        let trace = self.trace.clone();
        Listener::new(SizedBox::shrink())
            .behavior(HitTestBehavior::Translucent)
            .on_pointer_down(move |_| trace.borrow_mut().push(id))
            .into()
    }
}

impl Sliver for OverlappingSliver {
    fn build(&self, _controller: &ScrollController) -> Widget {
        self.listener()
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(self.clone())
    }
}

impl RenderSliver for OverlappingSliver {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        let extent = constraints.remaining_paint_extent;
        let size = Size::new(constraints.cross_axis_extent, extent);
        SliverLayout {
            geometry: SliverGeometry {
                paint_extent: extent,
                max_paint_extent: extent,
                hit_test_extent: extent,
                visible: extent > 0.,
                ..SliverGeometry::ZERO
            },
            children: vec![SliverChildLayout {
                id: SliverChildId(0),
                widget: self.listener(),
                semantic_index: None,
                offset: 0.,
                cross_offset: 0.,
                constraints: Constraints::tight(size),
                extent,
                placement: SliverChildPlacement::Flow,
            }],
            absorbed_overlap: 0.,
        }
    }
}

#[test]
fn centered_translucent_slivers_all_receive_hits_from_front_to_back() {
    for (order, expected) in [
        (None, [0, 1, 2, 3, 4]),
        (Some(SliverPaintOrder::LastIsTop), [4, 3, 2, 1, 0]),
    ] {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let slivers = (0..5)
            .map(|id| -> Box<dyn Sliver> {
                Box::new(OverlappingSliver {
                    id,
                    trace: trace.clone(),
                })
            })
            .collect();
        let mut tree = at_center(slivers, order);
        let _ = tree.dispatch_raw_pointer(RawPointerEvent::new(
            1,
            0,
            PointerDeviceKind::Touch,
            1,
            Offset::new(400., 300.),
            PointerPhase::Down,
            Instant::now(),
        ));
        assert_eq!(&*trace.borrow(), &expected, "{order:?}");
    }
}
