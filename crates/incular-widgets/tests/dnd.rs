//! Typed drag and drop: the dragged element lifts itself; targets use
//! `dnd.active()` / `dnd.over()` conditions and receive one drop.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{DndExt, DragDrop, Transition, ms};
use incular_widgets::{Directionality, Row, TextDirection, Widget};

struct Harness {
    tree: WidgetTree,
    now: Instant,
}

impl Harness {
    fn frame(&mut self) {
        self.tree.tick_animations(self.now);
        self.tree
            .layout(Constraints::loose(Size::new(400., 200.)))
            .expect("layout");
        self.tree.update_compositor(self.now).expect("compositor");
        let _ = self.tree.paint();
    }

    fn run(&mut self, duration: Duration) {
        let end = self.now + duration;
        while self.now < end {
            self.now += ms(16);
            self.frame();
        }
    }

    fn pointer(&mut self, position: Offset, phase: PointerPhase) {
        let buttons = u32::from(phase != PointerPhase::Up);
        let raw = RawPointerEvent::new(
            0,
            1,
            PointerDeviceKind::Mouse,
            buttons,
            position,
            phase,
            self.now,
        );
        let _ = self.tree.dispatch_raw_pointer_in_window(7, raw);
        self.tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position,
            phase,
            time: self.now,
        });
    }

    /// Scale of the `n`th motion transform in tree order.
    fn scale(&self, n: usize) -> f32 {
        let mut found = Vec::new();
        let mut stack = vec![self.tree.root().expect("root")];
        while let Some(id) = stack.pop() {
            if let Some(transform) = self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.content_transform(render))
            {
                found.push(transform);
            }
            let children: Vec<ElementId> = self.tree.children(id).unwrap_or(&[]).to_vec();
            stack.extend(children.into_iter().rev());
        }
        let [a, ..] = found[n].coefficients();
        a as f32
    }
}

#[test]
fn drops_reach_the_accepting_target_under_the_pointer() {
    let dnd = DragDrop::<u32>::new();
    let dropped = Rc::new(RefCell::new(Vec::new()));
    let sink = dropped.clone();
    let file = Widget::box_(Size::new(100., 100.), Color::WHITE)
        .draggable(&dnd, 7)
        .motion(|m| {
            m.dragging(|p| p.scale(1.1))
                .transition(Transition::instant())
        });
    let folder = Widget::box_(Size::new(100., 100.), Color::BLACK)
        .drop_target(&dnd, |t| {
            t.accepts(|id| *id == 7)
                .on_drop(move |id| sink.borrow_mut().push(id))
        })
        .motion(|m| {
            m.when(dnd.active(), |p| p.scale(1.02))
                .when(dnd.over(), |p| p.scale(1.2))
                .transition(Transition::instant())
        });
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Row::new([file, folder])).into())
        .expect("mount");
    let mut h = Harness {
        tree,
        now: Instant::now(),
    };
    h.frame();
    h.frame();
    h.pointer(Offset::new(50., 50.), PointerPhase::Down);
    for x in [60., 70., 80., 90.] {
        h.now += ms(16);
        h.pointer(Offset::new(x, 50.), PointerPhase::Move);
        h.frame();
    }
    h.run(ms(32));
    assert!((h.scale(0) - 1.1).abs() < 1e-3, "the file lifts itself");
    assert!(
        (h.scale(1) - 1.02).abs() < 1e-3,
        "an accepting target is active"
    );
    for x in [140., 160.] {
        h.now += ms(16);
        h.pointer(Offset::new(x, 50.), PointerPhase::Move);
        h.frame();
    }
    h.run(ms(32));
    assert!(
        (h.scale(1) - 1.2).abs() < 1e-3,
        "the target under the pointer is over"
    );
    h.pointer(Offset::new(160., 50.), PointerPhase::Up);
    h.run(ms(400));
    assert_eq!(*dropped.borrow(), vec![7]);
    assert!((h.scale(1) - 1.).abs() < 1e-3, "flags clear after the drop");
}
