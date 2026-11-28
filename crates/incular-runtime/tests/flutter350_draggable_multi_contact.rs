use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, InputEvent, Offset, PointerDeviceKind, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, DragDropContext, DragTarget, Draggable, Positioned, SizedBox, Stack, Widget,
    internal::Key,
};

type Drops = Rc<RefCell<Vec<(i32, Offset)>>>;

struct Host {
    runtime: Runtime,
    next: Instant,
    context: DragDropContext<i32>,
    data: Rc<RefCell<Vec<i32>>>,
    drops: Drops,
}

impl Host {
    fn mount(limit: usize) -> Self {
        let context = DragDropContext::new();
        let sequence = Rc::new(Cell::new(0));
        let data = Rc::new(RefCell::new(Vec::new()));
        let drops = Drops::default();
        let source = Draggable::new(
            context.clone(),
            1,
            Widget::box_(Size::new(84., 14.), Color::BLACK).with_key("source"),
        )
        .max_simultaneous_drags(limit)
        .feedback({
            let sequence = sequence.clone();
            move |_| {
                let number = sequence.get() + 1;
                sequence.set(number);
                Widget::box_(Size::new(40., 14.), Color::WHITE)
                    .with_key(Key::from(format!("feedback {number}").as_str()))
            }
        });
        let target = DragTarget::new(
            context.clone(),
            Widget::box_(Size::new(84., 100.), Color::BLACK).with_key("target"),
        )
        .on_drop({
            let data = data.clone();
            move |payload| data.borrow_mut().push(payload)
        })
        .on_accept_with_details({
            let drops = drops.clone();
            move |payload, offset| drops.borrow_mut().push((payload, offset))
        });
        let root = Column::new([Widget::from(source), Widget::from(target)])
            .text_direction(TextDirection::Ltr);
        let mut host = Self {
            runtime: Runtime::new(root.into()).unwrap(),
            next: Instant::now(),
            context,
            data,
            drops,
        };
        host.frame();
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.next)
            .unwrap();
        self.next += Duration::from_millis(16);
    }

    fn pointer(&mut self, pointer: u64, phase: PointerPhase, position: Offset) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                position,
                kind: PointerDeviceKind::Touch,
                phase,
                buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
                    0
                } else {
                    1
                },
                button: None,
                sample: Default::default(),
            },
            self.next,
        );
        self.frame();
    }

    fn origin(&self, key: &str) -> Option<Offset> {
        let tree = self.runtime.tree();
        tree.element_bounds(tree.element_with_key(&Key::from(key))?)
            .map(|rect| rect.origin)
    }
}

#[test]
fn max_simultaneous_drags_preserves_two_independent_avatars_and_drops() {
    let source = Offset::new(400., 7.);
    let target = Offset::new(400., 64.);
    let mut disabled = Host::mount(0);
    disabled.pointer(1, PointerPhase::Down, source);
    disabled.pointer(1, PointerPhase::Up, source);
    assert!(!disabled.context.is_dragging());
    assert_eq!(disabled.origin("feedback 1"), None);
    assert!(disabled.data.borrow().is_empty());

    let mut host = Host::mount(2);
    for pointer in [1, 2, 3] {
        host.pointer(pointer, PointerPhase::Down, source);
    }
    assert_eq!(host.origin("source"), Some(Offset::new(358., 0.)));
    assert_eq!(host.origin("feedback 1"), Some(Offset::new(358., 0.)));
    assert_eq!(host.origin("feedback 2"), Some(Offset::new(358., 0.)));
    assert_eq!(host.origin("feedback 3"), None);

    host.pointer(1, PointerPhase::Move, Offset::new(380., 45.));
    host.pointer(2, PointerPhase::Move, Offset::new(430., 75.));
    assert_eq!(host.origin("feedback 1"), Some(Offset::new(338., 38.)));
    assert_eq!(host.origin("feedback 2"), Some(Offset::new(388., 68.)));
    for pointer in [1, 2, 3] {
        host.pointer(pointer, PointerPhase::Move, target);
    }
    assert!(host.drops.borrow().is_empty());
    host.pointer(1, PointerPhase::Up, target);
    assert_eq!(&*host.data.borrow(), &[1]);
    assert_eq!(&*host.drops.borrow(), &[(1, Offset::new(358., 57.))]);
    assert!(host.context.is_dragging());
    assert_eq!(host.origin("feedback 1"), None);
    assert_eq!(host.origin("feedback 2"), Some(Offset::new(358., 57.)));

    host.pointer(2, PointerPhase::Up, target);
    host.pointer(3, PointerPhase::Up, target);
    assert_eq!(&*host.data.borrow(), &[1, 1]);
    assert_eq!(
        &*host.drops.borrow(),
        &[(1, Offset::new(358., 57.)), (1, Offset::new(358., 57.))]
    );
    assert!(!host.context.is_dragging());
    assert_eq!(host.origin("feedback 2"), None);
}

#[test]
fn canceling_one_source_keeps_the_other_sources_feedback_and_candidate_payload() {
    let context = DragDropContext::new();
    let leaves = Rc::new(RefCell::new(Vec::new()));
    let candidates = Rc::new(RefCell::new(Vec::new()));
    let data = Rc::new(RefCell::new(Vec::new()));
    let drops = Drops::default();
    let source = |value, x| {
        Positioned::new(
            Draggable::new(
                context.clone(),
                value,
                Widget::box_(Size::new(80., 40.), Color::BLACK),
            )
            .feedback(move |_| {
                Widget::box_(Size::new(40., 20.), Color::WHITE)
                    .with_key(Key::from(format!("feedback {value}").as_str()))
            }),
        )
        .left(x)
        .top(20.)
    };
    let target = Positioned::new(
        DragTarget::builder(context.clone(), {
            let candidates = candidates.clone();
            move |accepted, _| {
                *candidates.borrow_mut() = accepted;
                SizedBox::expand().into()
            }
        })
        .on_leave({
            let leaves = leaves.clone();
            move |payload| leaves.borrow_mut().push(payload)
        })
        .on_accept_with_details({
            let drops = drops.clone();
            move |payload, point| drops.borrow_mut().push((payload, point))
        }),
    )
    .left(20.)
    .top(150.)
    .width(300.)
    .height(100.);
    let mut host = Host {
        runtime: Runtime::new(Stack::new([source(7, 20.), source(9, 120.), target]).into())
            .unwrap(),
        next: Instant::now(),
        context,
        data,
        drops,
    };
    host.frame();
    host.pointer(1, PointerPhase::Down, Offset::new(60., 40.));
    host.pointer(2, PointerPhase::Down, Offset::new(160., 40.));
    host.pointer(1, PointerPhase::Move, Offset::new(90., 180.));
    host.pointer(2, PointerPhase::Move, Offset::new(170., 210.));
    assert_eq!(&*candidates.borrow(), &[7, 9]);
    assert_eq!(host.origin("feedback 7"), Some(Offset::new(50., 160.)));
    assert_eq!(host.origin("feedback 9"), Some(Offset::new(130., 190.)));

    host.pointer(1, PointerPhase::Cancel, Offset::new(90., 180.));
    assert_eq!(host.origin("feedback 7"), None);
    assert_eq!(host.origin("feedback 9"), Some(Offset::new(130., 190.)));
    assert_eq!(&*candidates.borrow(), &[9]);
    assert_eq!(&*leaves.borrow(), &[7]);
    assert!(host.drops.borrow().is_empty());

    host.pointer(2, PointerPhase::Up, Offset::new(170., 210.));
    assert!(candidates.borrow().is_empty());
    assert_eq!(&*host.drops.borrow(), &[(9, Offset::new(130., 190.))]);
    assert_eq!(&*leaves.borrow(), &[7]);
    assert_eq!(host.origin("feedback 9"), None);
    assert!(!host.context.is_dragging());
}
