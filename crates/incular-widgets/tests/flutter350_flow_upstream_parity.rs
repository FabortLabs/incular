//! Flow paint scheduling, input order, opacity and degenerate geometry.
use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size, Transform};
use incular_gestures::PointerEvent;
use incular_rendering::PaintCommand;
use incular_widgets::{
    Flow, FlowDelegate, FlowPaintingContext, GestureDetector, Placeholder, Widget,
    internal::WidgetTree,
};
use std::{
    cell::{Cell, RefCell},
    panic::AssertUnwindSafe,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Layout {
    shift: Rc<Cell<f32>>,
    order: Vec<usize>,
    spacing: f32,
    alpha: f32,
}
impl FlowDelegate for Layout {
    fn get_constraints_for_child(&self, _: usize, constraints: Constraints) -> Constraints {
        constraints.loosen()
    }
    fn paint_children(&self, ctx: &mut FlowPaintingContext<'_>) {
        let mut y = self.shift.get();
        for &id in &self.order {
            ctx.paint_child(id, Transform::translation(Offset::new(0., y)), self.alpha);
            y += self.spacing * ctx.get_child_size(id).unwrap().height;
        }
    }
    fn should_repaint(&self, old: &Self) -> bool {
        !Rc::ptr_eq(&self.shift, &old.shift) || self.order != old.order || self.alpha != old.alpha
    }
}

fn layout(tree: &mut WidgetTree, size: Size) {
    tree.layout(Constraints::tight(size)).unwrap();
    let _ = tree.paint();
}
fn clickable(index: usize, log: Rc<RefCell<Vec<usize>>>) -> Widget {
    let color = if index == 0 {
        Color::WHITE
    } else {
        Color::BLACK
    };
    GestureDetector::new(Widget::box_(Size::new(100., 100.), color))
        .on_tap(move || log.borrow_mut().push(index))
        .into()
}
fn tap(tree: &mut WidgetTree, point: Offset, start: Instant, number: u64) {
    for (phase, delay) in [(PointerPhase::Down, 0), (PointerPhase::Up, 8)] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position: point,
            phase,
            time: start + Duration::from_millis(number * 40 + delay),
        });
    }
}

fn flow(shift: Rc<Cell<f32>>, order: Vec<usize>, spacing: f32, children: Vec<Widget>) -> Widget {
    Flow::new(
        Layout {
            shift,
            order,
            spacing,
            alpha: 1.,
        },
        children,
    )
    .into()
}

#[test]
fn flow_repaint_moves_overlap_hit_without_replacing_the_delegate() {
    let hits = Rc::new(RefCell::new(Vec::new()));
    let shift = Rc::new(Cell::new(0.));
    let children = (0..7)
        .map(|i| clickable(i, hits.clone()))
        .collect::<Vec<_>>();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(flow(
            shift.clone(),
            (0..7).collect(),
            0.75,
            children.clone(),
        ))
        .unwrap();
    let viewport = Size::new(800., 600.);
    layout(&mut tree, viewport);
    let clock = Instant::now();
    for (step, y) in [50., 125., 200., 90.].into_iter().enumerate() {
        tap(&mut tree, Offset::new(20., y), clock, step as u64);
    }
    assert_eq!(hits.take(), [0, 1, 2, 1]);

    shift.set(50.);
    layout(&mut tree, viewport);
    tap(&mut tree, Offset::new(20., 90.), clock, 5);
    let same_delegate = hits.take();
    tree.update(
        root,
        flow(Rc::new(Cell::new(50.)), (0..7).collect(), 0.75, children),
    )
    .unwrap();
    layout(&mut tree, viewport);
    tap(&mut tree, Offset::new(20., 90.), clock, 6);
    assert_eq!(hits.take(), [0], "explicit new delegate control");
    assert_eq!(
        same_delegate,
        [0],
        "pinned delegate's repaint Listenable does not need replacement"
    );
}

#[test]
fn duplicate_child_paint_reports_the_child_and_rejects_the_frame() {
    let mut tree = WidgetTree::new();
    let descriptor = Layout {
        shift: Rc::new(Cell::new(0.)),
        order: vec![0, 1, 0],
        spacing: 0.,
        alpha: 1.,
    };
    let box_ = || Widget::box_(Size::new(100., 100.), Color::WHITE);
    tree.mount(Flow::new(descriptor, [box_(), box_()]).into())
        .unwrap();
    let err = std::panic::catch_unwind(AssertUnwindSafe(|| {
        layout(&mut tree, Size::new(800., 600.))
    }))
    .expect_err("duplicate paint must report an error");
    let message = err
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(message.contains("AlreadyPainted(0)"), "{message}");
}

#[test]
fn flow_child_is_painted_with_requested_twenty_percent_opacity() {
    let mut tree = WidgetTree::new();
    let descriptor = Layout {
        shift: Rc::new(Cell::new(0.)),
        order: vec![0],
        spacing: 0.,
        alpha: 0.2,
    };
    tree.mount(
        Flow::new(
            descriptor,
            [Widget::box_(Size::new(100., 100.), Color::WHITE)],
        )
        .into(),
    )
    .unwrap();
    layout(&mut tree, Size::new(800., 600.));
    let picture = tree.paint();
    let alphas: Vec<_> = picture
        .commands()
        .iter()
        .filter_map(|c| match c {
            PaintCommand::PushOpacity { alpha, .. } => Some(*alpha),
            _ => None,
        })
        .collect();
    assert_eq!(alphas, [0.2]);
}

#[test]
fn flow_zero_area_lays_out_and_paints_its_placeholder() {
    let mut tree = WidgetTree::new();
    let descriptor = Layout {
        shift: Rc::new(Cell::new(0.)),
        order: vec![0],
        spacing: 0.,
        alpha: 0.2,
    };
    let root = tree
        .mount(Flow::new(descriptor, [Placeholder::new()]).into())
        .unwrap();
    layout(&mut tree, Size::ZERO);
    assert_eq!(tree.element_bounds(root).unwrap().size, Size::ZERO);
}

#[test]
fn overlap_dispatches_to_the_last_child_in_delegate_paint_order() {
    let mut seen = Vec::new();
    for order in [vec![0, 1], vec![1, 0]] {
        let hits = Rc::new(RefCell::new(Vec::new()));
        let mut tree = WidgetTree::new();
        tree.mount(flow(
            Rc::new(Cell::new(0.)),
            order,
            0.,
            (0..2).map(|i| clickable(i, hits.clone())).collect(),
        ))
        .unwrap();
        layout(&mut tree, Size::new(800., 600.));
        tap(&mut tree, Offset::new(20., 20.), Instant::now(), 1);
        seen.push(hits.take());
    }
    assert_eq!(seen, [vec![1], vec![0]]);
}
