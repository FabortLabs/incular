use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::AnimationController;
use incular_config::Constraints;
use incular_core::{
    ChangeNotifier, Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind,
    PointerPhase, PointerSampleMetadata, Size, Transform, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Flow, FlowDelegate, FlowPaintingContext, GestureDetector, Widget, internal::Key,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

struct Moving {
    offset: Rc<dyn Fn() -> f32>,
    paints: Rc<Cell<usize>>,
}
impl FlowDelegate for Moving {
    fn get_constraints_for_child(&self, _: usize, constraints: Constraints) -> Constraints {
        constraints.loosen()
    }

    fn paint_children(&self, context: &mut FlowPaintingContext<'_>) {
        self.paints.set(self.paints.get() + 1);
        let shift = (self.offset)();
        for index in 0..context.child_count() {
            context.paint_child(
                index,
                Transform::translation(Offset::new(0., shift + 40. * index as f32)),
                1.,
            );
        }
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

fn clickable(index: usize, hits: Rc<RefCell<Vec<usize>>>) -> Widget {
    Widget::from(
        GestureDetector::new(Widget::box_(Size::new(80., 60.), Color::WHITE))
            .on_tap(move || hits.borrow_mut().push(index)),
    )
    .with_key(Key::Value(index as u64))
}

fn tap(runtime: &mut Runtime, now: Instant) {
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons,
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position: Offset::new(20., 50.),
            },
            now,
        );
    }
}

#[test]
fn a_repaint_source_requests_a_frame_without_remeasuring_and_detaches_on_replacement() {
    let first = ChangeNotifier::new();
    let second = ChangeNotifier::new();
    let shift = Rc::new(Cell::new(0.));
    let hits = Rc::new(RefCell::new(Vec::new()));
    let paints = Rc::new(Cell::new(0));
    let buttons = [clickable(0, hits.clone()), clickable(1, hits.clone())];
    let build = |source: &ChangeNotifier| {
        let shift = shift.clone();
        Widget::from(
            Flow::new(
                Moving {
                    offset: Rc::new(move || shift.get()),
                    paints: paints.clone(),
                },
                buttons.clone(),
            )
            .repaint(source.clone()),
        )
    };
    let mut runtime = Runtime::new(build(&first)).unwrap();
    let bounds = Constraints::tight(VIEW);
    let now = Instant::now();
    runtime.run_frame_at(bounds, now).unwrap();
    assert_eq!(paints.get(), 1);
    assert!(first.has_listeners());
    tap(&mut runtime, now);
    assert_eq!(*hits.borrow(), [1]);

    runtime
        .run_frame_at(bounds, now + Duration::from_millis(1))
        .unwrap();
    let before = runtime.tree().diagnostics();
    shift.set(30.);
    first.notify_listeners();
    assert!(runtime.frame_requested());
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(2))
        .unwrap();
    let after = runtime.tree().diagnostics();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert_eq!(after.layouts, before.layouts);
    assert_eq!(paints.get(), 2);
    tap(&mut runtime, now + Duration::from_millis(3));
    assert_eq!(*hits.borrow(), [1, 0]);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(&second)).unwrap();
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(4))
        .unwrap();
    assert!(!first.has_listeners());
    assert!(second.has_listeners());
    let preserved = paints.get();
    first.notify_listeners();
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(5))
        .unwrap();
    assert_eq!(paints.get(), preserved);
    second.notify_listeners();
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(6))
        .unwrap();
    assert_eq!(paints.get(), preserved + 1);
}

#[test]
fn an_animation_is_ticked_as_a_flow_repaint_source_without_widget_rebuilds() {
    let animation = AnimationController::new(Duration::from_millis(100)).with_vsync();
    let hits = Rc::new(RefCell::new(Vec::new()));
    let paints = Rc::new(Cell::new(0));
    let values = Rc::new(RefCell::new(Vec::<f32>::new()));
    let flow = Flow::new(
        Moving {
            offset: Rc::new({
                let animation = animation.clone();
                let values = values.clone();
                move || {
                    let value = animation.value();
                    values.borrow_mut().push(value);
                    value * 100.
                }
            }),
            paints: paints.clone(),
        },
        [clickable(0, hits.clone()), clickable(1, hits.clone())],
    )
    .repaint(animation.clone());
    let mut runtime = Runtime::new(flow.into()).unwrap();
    let bounds = Constraints::tight(VIEW);
    let now = Instant::now();
    runtime.run_frame_at(bounds, now).unwrap();
    let before = runtime.tree().diagnostics();
    assert_eq!(paints.get(), 1);
    let _ = animation.forward(now + Duration::from_millis(1));
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(1))
        .unwrap();
    runtime
        .run_frame_at(bounds, now + Duration::from_millis(31))
        .unwrap();
    assert!((animation.value() - 0.3).abs() < 0.00001);
    let after = runtime.tree().diagnostics();
    assert_eq!(after.rebuilds, before.rebuilds);
    assert_eq!(after.layouts, before.layouts);
    assert_eq!(paints.get(), 3);
    let samples = values.borrow();
    assert_eq!(&samples[..2], [0., 0.]);
    assert!((samples[2] - 0.3).abs() < 0.00001);
    drop(samples);
    tap(&mut runtime, now + Duration::from_millis(32));
    assert_eq!(*hits.borrow(), [0]);
}
