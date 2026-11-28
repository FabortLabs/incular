use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_text::TextStyle;
use incular_widgets::{
    Column, Directionality, DragDropContext, DragTarget, Draggable, SizedBox, Text, Widget,
    internal::Key,
};

const FONT: &str = "Embedded FlutterTest DragTarget";

struct Host {
    runtime: Runtime,
    clock: Instant,
    context: DragDropContext<i32>,
    starts: Rc<Cell<usize>>,
    moves: Rc<RefCell<Vec<(&'static str, i32, Offset)>>>,
    leaves: Rc<RefCell<Vec<(&'static str, i32)>>>,
    dropped: Rc<RefCell<Vec<(i32, Offset)>>>,
    drop_data: Rc<RefCell<Vec<i32>>>,
}

impl Host {
    fn mount(two_targets: bool) -> Option<Self> {
        let font = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH")?;
        incular_text::register_application_font_as(FONT, std::fs::read(font).unwrap());
        let context = DragDropContext::new();
        let starts = Rc::new(Cell::new(0));
        let moves = Rc::new(RefCell::new(Vec::new()));
        let leaves = Rc::new(RefCell::new(Vec::new()));
        let dropped = Rc::new(RefCell::new(Vec::new()));
        let drop_data = Rc::new(RefCell::new(Vec::new()));
        fn label(text: &str) -> Widget {
            Text::new(text)
                .style(
                    TextStyle::new()
                        .font_family(FONT)
                        .font_size(14.)
                        .height(Some(1.)),
                )
                .into()
        }
        let source = Draggable::new(context.clone(), 1, label("Source").with_key("source"))
            .feedback(|_| label("Dragging").with_key("feedback"))
            .on_start({
                let starts = starts.clone();
                move |_| starts.set(starts.get() + 1)
            });
        let target = |name| {
            DragTarget::builder(context.clone(), move |_, _| {
                SizedBox::new()
                    .height(100.)
                    .child(label(name).with_key(name))
                    .into()
            })
            .on_update({
                let log = moves.clone();
                move |data, offset| log.borrow_mut().push((name, data, offset))
            })
            .on_leave({
                let log = leaves.clone();
                move |data| log.borrow_mut().push((name, data))
            })
            .on_drop({
                let log = drop_data.clone();
                move |data| log.borrow_mut().push(data)
            })
            .on_accept_with_details({
                let log = dropped.clone();
                move |data, offset| log.borrow_mut().push((data, offset))
            })
        };
        let mut children = vec![Widget::from(source), target("Target 1").into()];
        if two_targets {
            children.push(target("Target 2").into());
        }
        let root = Directionality::new(TextDirection::Ltr, Column::new(children));
        let mut host = Self {
            runtime: Runtime::new(root.into()).unwrap(),
            clock: Instant::now(),
            context,
            starts,
            moves,
            leaves,
            dropped,
            drop_data,
        };
        host.frame();
        Some(host)
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.clock)
            .unwrap();
        self.clock += Duration::from_millis(16);
    }

    fn center(&self, key: &str) -> Offset {
        let tree = self.runtime.tree();
        let bounds = tree
            .element_bounds(tree.element_with_key(&Key::from(key)).unwrap())
            .unwrap();
        bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.)
    }

    fn send(&mut self, phase: PointerPhase, point: Offset) {
        let _ = self.runtime.handle_input_at(
            InputEvent::Pointer {
                phase,
                position: point,
            },
            self.clock,
        );
        self.frame();
    }
}

#[test]
fn source_starts_at_press_and_drops_once_at_the_original_child_offset() {
    let Some(mut host) = Host::mount(false) else {
        return;
    };
    let first = host.center("source");
    let target = host.center("Target 1");
    assert_eq!(host.starts.get(), 0);
    assert!(host.context.feedback().is_none());
    host.send(PointerPhase::Down, first);
    assert_eq!(host.starts.get(), 1);
    assert!(host.context.feedback().is_some());
    assert!(host.moves.borrow().is_empty());
    assert!(host.dropped.borrow().is_empty());
    host.send(PointerPhase::Move, target);
    assert_eq!(host.moves.borrow().len(), 1);
    assert!(host.dropped.borrow().is_empty());
    host.send(PointerPhase::Up, target);
    assert!(host.context.feedback().is_none());
    assert_eq!(host.starts.get(), 1);
    assert_eq!(host.moves.borrow().len(), 1);
    assert_eq!(&*host.drop_data.borrow(), &[1]);
    assert_eq!(&*host.dropped.borrow(), &[(1, Offset::new(358., 57.))]);
}

#[test]
fn target_move_and_leave_follow_each_of_two_real_typed_targets() {
    let Some(mut host) = Host::mount(true) else {
        return;
    };
    let source = host.center("source");
    let first = host.center("Target 1");
    let second = host.center("Target 2");
    host.send(PointerPhase::Down, source);
    assert!(host.moves.borrow().is_empty() && host.leaves.borrow().is_empty());
    host.send(PointerPhase::Move, first);
    assert_eq!(host.moves.borrow().len(), 1);
    assert!(host.leaves.borrow().is_empty());
    host.send(PointerPhase::Move, second);
    assert_eq!(host.moves.borrow().len(), 2);
    assert_eq!(&*host.leaves.borrow(), &[("Target 1", 1)]);
    host.send(PointerPhase::Move, first);
    assert_eq!(
        host.moves
            .borrow()
            .iter()
            .map(|(name, data, _)| (*name, *data))
            .collect::<Vec<_>>(),
        [("Target 1", 1), ("Target 2", 1), ("Target 1", 1)]
    );
    assert_eq!(&*host.leaves.borrow(), &[("Target 1", 1), ("Target 2", 1)]);
    host.send(PointerPhase::Up, first);
    assert_eq!(host.moves.borrow().len(), 3);
    assert_eq!(host.leaves.borrow().len(), 2);
    assert_eq!(&*host.drop_data.borrow(), &[1]);
    assert_eq!(&*host.dropped.borrow(), &[(1, Offset::new(358., 57.))]);
}
