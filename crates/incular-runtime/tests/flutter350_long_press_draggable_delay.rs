use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, DragDropContext, DragTarget, LongPressDraggable, Widget, internal::Key,
};

struct Host {
    runtime: Runtime,
    starts: Rc<Cell<usize>>,
    drops: Rc<RefCell<Vec<&'static str>>>,
    begin: Instant,
}

impl Host {
    fn mount(delay: Option<Duration>) -> Self {
        let starts = Rc::new(Cell::new(0));
        let drops = Rc::new(RefCell::new(Vec::new()));
        let context = DragDropContext::new();
        let mut source = LongPressDraggable::new(
            context.clone(),
            1,
            Widget::box_(Size::new(80., 30.), Color::WHITE).with_key("source"),
        )
        .feedback(|_| Widget::box_(Size::new(80., 30.), Color::BLACK).with_key("feedback"))
        .on_start({
            let starts = starts.clone();
            move |_| starts.set(starts.get() + 1)
        });
        if let Some(delay) = delay {
            source = source.delay(delay);
        }
        let target = DragTarget::new(
            context,
            Widget::box_(Size::new(80., 100.), Color::WHITE).with_key("target"),
        )
        .on_drop({
            let log = drops.clone();
            move |_| log.borrow_mut().push("drop")
        })
        .on_accept_with_details({
            let log = drops.clone();
            move |_, _| log.borrow_mut().push("details")
        });
        let root = Column::new([Widget::from(source), Widget::from(target)])
            .text_direction(TextDirection::Ltr);
        let mut host = Self {
            runtime: Runtime::new(root.into()).unwrap(),
            starts,
            drops,
            begin: Instant::now(),
        };
        host.frame(0);
        host
    }

    fn frame(&mut self, elapsed: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(Size::new(320., 240.)),
                self.begin + Duration::from_millis(elapsed),
            )
            .unwrap();
    }

    fn pointer(&mut self, phase: PointerPhase, point: Offset, elapsed: u64) {
        let _ = self.runtime.handle_input_at(
            InputEvent::Pointer {
                phase,
                position: point,
            },
            self.begin + Duration::from_millis(elapsed),
        );
        self.frame(elapsed);
    }

    fn exists(&self, key: &str) -> bool {
        self.runtime
            .tree()
            .element_with_key(&Key::from(key))
            .is_some()
    }
}

const SOURCE: Offset = Offset::new(160., 15.);
const TARGET: Offset = Offset::new(160., 80.);

#[test]
fn default_and_custom_delays_start_feedback_at_the_exact_deadline() {
    for (delay, millis) in [(None, 500), (Some(Duration::from_secs(2)), 2000)] {
        let mut host = Host::mount(delay);
        assert!(host.exists("source"));
        assert!(!host.exists("feedback"));
        assert_eq!(host.starts.get(), 0);
        host.pointer(PointerPhase::Down, SOURCE, 10);
        for elapsed in [0, millis / 2, millis - 1] {
            host.frame(10 + elapsed);
            assert!(host.exists("source"));
            assert!(!host.exists("feedback"));
            assert_eq!(host.starts.get(), 0);
        }
        host.frame(10 + millis);
        assert!(host.exists("source") && host.exists("feedback"));
        assert_eq!(host.starts.get(), 1);
        host.pointer(PointerPhase::Up, SOURCE, 20 + millis);
        assert!(!host.exists("feedback"));
    }
}

#[test]
fn a_short_press_or_move_outside_slop_before_the_deadline_never_drops() {
    let mut host = Host::mount(None);
    host.pointer(PointerPhase::Down, SOURCE, 10);
    host.pointer(PointerPhase::Up, SOURCE, 20);
    host.pointer(PointerPhase::Down, SOURCE, 30);
    host.pointer(PointerPhase::Move, TARGET, 40);
    host.frame(1500);
    assert!(host.exists("source") && host.exists("target") && !host.exists("feedback"));
    assert_eq!(host.starts.get(), 0);
    host.pointer(PointerPhase::Up, TARGET, 1510);
    assert!(host.drops.borrow().is_empty());
}
