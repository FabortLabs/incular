use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Axis, Constraints};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    DragDropContext, Draggable, Positioned, SizedBox, Stack, Text, Widget, internal::Key,
};

struct Scene {
    runtime: Runtime,
    context: DragDropContext<i32>,
    updates: Rc<RefCell<Vec<Offset>>>,
    now: Instant,
}

impl Scene {
    fn new(axis: Option<Axis>, swap_child: bool) -> Self {
        let updates = Rc::new(RefCell::new(Vec::new()));
        let context = DragDropContext::new();
        let mut drag = Draggable::new(
            context.clone(),
            1,
            Widget::from(Text::new("source").text_direction(incular_widgets::TextDirection::Ltr))
                .with_key("source"),
        )
        .feedback(|_| {
            Widget::from(Text::new("feedback").text_direction(incular_widgets::TextDirection::Ltr))
                .with_key("feedback")
        })
        .on_drag_update({
            let updates = updates.clone();
            move |delta| updates.borrow_mut().push(delta)
        });
        if swap_child {
            drag = drag.child_when_dragging(SizedBox::shrink());
        }
        if let Some(axis) = axis {
            drag = drag.axis(axis);
        }
        let mut scene = Self {
            runtime: Runtime::new(Stack::new([Positioned::new(drag).left(350.).top(250.)]).into())
                .unwrap(),
            context,
            updates,
            now: Instant::now(),
        };
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }

    fn input(&mut self, phase: PointerPhase, position: Offset) {
        let _ = self
            .runtime
            .handle_input_at(InputEvent::Pointer { phase, position }, self.now);
        self.frame();
    }

    fn bounds(&self, key: &str) -> Option<incular_core::Rect> {
        let tree = self.runtime.tree();
        tree.element_bounds(tree.element_with_key(&Key::from(key))?)
    }

    fn visible_feedback_at(&self, expected: Offset) {
        assert_eq!(self.context.feedback().unwrap().position, expected);
        assert_eq!(self.bounds("feedback").unwrap().origin, expected);
    }
}

#[test]
fn replacement_feedback_follows_unrestricted_and_axis_locked_positions() {
    for (axis, moved, filtered) in [
        (None, Offset::new(300., 300.), Offset::new(300., 300.)),
        (
            Some(Axis::Horizontal),
            Offset::new(300., 0.),
            Offset::new(300., 0.),
        ),
        (
            Some(Axis::Horizontal),
            Offset::new(300., 200.),
            Offset::new(300., 0.),
        ),
        (
            Some(Axis::Vertical),
            Offset::new(0., 300.),
            Offset::new(0., 300.),
        ),
        (
            Some(Axis::Vertical),
            Offset::new(200., 300.),
            Offset::new(0., 300.),
        ),
    ] {
        let mut scene = Scene::new(axis, true);
        let origin = scene.bounds("source").unwrap().origin;
        scene.input(PointerPhase::Down, origin);
        scene.visible_feedback_at(origin);
        assert!(scene.bounds("source").is_none());
        for direction in [1., -1.] {
            scene.input(
                PointerPhase::Move,
                origin + Offset::new(moved.x * direction, moved.y * direction),
            );
            scene.visible_feedback_at(
                origin + Offset::new(filtered.x * direction, filtered.y * direction),
            );
        }
        scene.input(PointerPhase::Up, origin - moved);
        assert!(scene.bounds("feedback").is_none());
        assert_eq!(scene.bounds("source").unwrap().origin, origin);
    }
}

#[test]
fn drag_update_only_fires_for_kept_nonzero_delta() {
    for (axis, first, second, expected) in [
        (
            None,
            Offset::new(10., 10.),
            Offset::ZERO,
            Offset::new(10., 10.),
        ),
        (
            Some(Axis::Vertical),
            Offset::new(0., 10.),
            Offset::new(10., 0.),
            Offset::new(0., 10.),
        ),
        (
            Some(Axis::Horizontal),
            Offset::new(0., 10.),
            Offset::new(10., 0.),
            Offset::new(10., 0.),
        ),
    ] {
        let mut scene = Scene::new(axis, false);
        let origin = scene.bounds("source").unwrap().origin;
        let size = scene.bounds("source").unwrap().size;
        let center = origin + Offset::new(size.width / 2., size.height / 2.);
        assert!(scene.bounds("feedback").is_none());
        scene.input(PointerPhase::Down, center);
        scene.visible_feedback_at(origin);
        assert!(scene.updates.borrow().is_empty());
        scene.input(PointerPhase::Move, center + first);
        assert_eq!(
            scene.updates.borrow().len(),
            usize::from(axis != Some(Axis::Horizontal))
        );
        scene.input(PointerPhase::Move, center + first + second);
        assert_eq!(&*scene.updates.borrow(), &[expected]);
        scene.visible_feedback_at(origin + expected);
        scene.input(PointerPhase::Up, center + first + second);
        assert_eq!(&*scene.updates.borrow(), &[expected]);
        assert!(scene.bounds("source").is_some() && scene.bounds("feedback").is_none());
    }
}
