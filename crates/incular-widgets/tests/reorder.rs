//! Reorder: measured-rect insertion, compositor sibling shifts, one
//! `on_move` per drop, FLIP absorption, keyboard and Kanban groups.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{
    Code, Color, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
    Transform as CoreTransform,
};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::motion::{
    Collision, Move, Placeholder, ReorderExt, ReorderGroup, Reorderable, ms,
};
use incular_widgets::{
    Column, Directionality, Row, ScrollController, SingleChildScrollView, SizedBox, TextDirection,
    Widget,
};

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    now: Instant,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            root,
            now: Instant::now(),
        };
        harness.frame();
        harness.frame();
        harness
    }

    fn frame(&mut self) {
        self.tree.tick_animations(self.now);
        self.tree
            .layout(Constraints::loose(Size::new(400., 400.)))
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

    fn update(&mut self, widget: Widget) {
        self.tree.update(self.root, widget).expect("update");
        self.frame();
    }

    fn pointer(&mut self, position: Offset, phase: PointerPhase) {
        self.pointer_of(PointerDeviceKind::Mouse, position, phase);
    }

    fn pointer_of(&mut self, kind: PointerDeviceKind, position: Offset, phase: PointerPhase) {
        let buttons = u32::from(phase != PointerPhase::Up);
        let raw = RawPointerEvent::new(0, 1, kind, buttons, position, phase, self.now);
        let _ = self.tree.dispatch_raw_pointer_in_window(7, raw);
        self.tree.dispatch_pointer_gesture_in_window(
            0,
            0,
            kind,
            incular_gestures::PointerEvent {
                pointer: 1,
                position,
                phase,
                time: self.now,
            },
        );
    }

    /// Drags from `from` to `to` in small steps, a frame each.
    fn drag(&mut self, from: Offset, to: Offset) {
        self.pointer(from, PointerPhase::Down);
        for step in 1..=10 {
            let t = step as f32 / 10.;
            self.now += ms(16);
            let at = Offset::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            self.pointer(at, PointerPhase::Move);
            self.frame();
        }
    }

    fn release(&mut self, at: Offset) {
        self.pointer(at, PointerPhase::Up);
        self.frame();
    }

    fn transforms(&self) -> Vec<CoreTransform> {
        let mut found = Vec::new();
        let mut stack = vec![self.root];
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
        found
    }

    /// The `n`th item's on-screen top: layout y plus its motion offset.
    fn visual_y(&self, n: usize) -> f32 {
        self.transforms()[n].translation_offset().y
    }
}

fn row(key: &'static str) -> Widget {
    Widget::box_(Size::new(100., 50.), Color::WHITE)
        .with_key(key)
        .reorder_item(key)
        .into()
}

fn list(order: &[&'static str], moves: &Rc<RefCell<Vec<Move<&'static str>>>>) -> Widget {
    let sink = moves.clone();
    Column::new(order.iter().map(|key| row(key)))
        .reorderable(move |mv| sink.borrow_mut().push(mv))
        .into()
}

#[test]
fn dragging_shifts_siblings_and_moves_once_on_drop() {
    let moves = Rc::new(RefCell::new(Vec::new()));
    let mut h = Harness::new(list(&["a", "b", "c"], &moves));
    let before = h.tree.diagnostics();
    // a's center (50, 25) down past b's center (75): a takes b's slot.
    h.drag(Offset::new(50., 25.), Offset::new(50., 85.));
    h.run(ms(400));
    let during = h.tree.diagnostics();
    assert_eq!(
        during.rebuilds, before.rebuilds,
        "no rebuilds while dragging"
    );
    assert!(
        (h.visual_y(1) + 50.).abs() < 1.,
        "b moved up into a's slot: {}",
        h.visual_y(1)
    );
    assert!(h.visual_y(2).abs() < 1., "c stays");
    assert!(moves.borrow().is_empty(), "nothing committed yet");
    h.release(Offset::new(50., 85.));
    assert_eq!(
        *moves.borrow(),
        vec![Move {
            key: "a",
            from: 0,
            to: 1,
            from_list: (),
            to_list: ()
        }]
    );
    // The app applies the move: offsets are absorbed and settle to rest.
    h.update(list(&["b", "a", "c"], &moves));
    let first = h.visual_y(0);
    assert!(first.abs() < 2., "b stays where it was shown: {first}");
    h.run(ms(600));
    assert!(
        h.transforms()
            .iter()
            .all(|t| t.translation_offset().y.abs() < 0.5)
    );
    assert_eq!(moves.borrow().len(), 1, "on_move ran once");
}

#[test]
fn an_ignored_move_relaxes_home() {
    let moves = Rc::new(RefCell::new(Vec::new()));
    let mut h = Harness::new(list(&["a", "b", "c"], &moves));
    h.drag(Offset::new(50., 25.), Offset::new(50., 85.));
    h.run(ms(300));
    h.release(Offset::new(50., 85.));
    h.run(ms(800));
    assert!(
        h.transforms()
            .iter()
            .all(|t| t.translation_offset().y.abs() < 0.5),
        "everything returns when the app keeps its order"
    );
}

#[test]
fn keyboard_lifts_moves_and_drops() {
    let moves = Rc::new(RefCell::new(Vec::new()));
    let mut h = Harness::new(list(&["a", "b", "c"], &moves));
    let target = h.tree.focusable_elements()[0];
    h.tree.set_keyboard_focus(target, true);
    let key = |h: &mut Harness, key: KeyboardKey, code: Code| {
        let _ = h
            .tree
            .dispatch_keyboard(Some(target), KeyboardEvent::key_down(key.clone(), code));
        let _ = h
            .tree
            .dispatch_keyboard(Some(target), KeyboardEvent::key_up(key, code));
        h.run(ms(32));
    };
    key(&mut h, KeyboardKey::Character(" ".into()), Code::Space);
    key(
        &mut h,
        KeyboardKey::Named(NamedKey::ArrowDown),
        Code::ArrowDown,
    );
    key(
        &mut h,
        KeyboardKey::Named(NamedKey::ArrowDown),
        Code::ArrowDown,
    );
    h.run(ms(900));
    assert!(
        (h.visual_y(0) - 100.).abs() < 1.,
        "a shown in the last slot: {}",
        h.visual_y(0)
    );
    key(&mut h, KeyboardKey::Character(" ".into()), Code::Space);
    assert_eq!(moves.borrow().len(), 1);
    assert_eq!(moves.borrow()[0].to, 2);
}

#[test]
fn escape_cancels_a_keyboard_lift() {
    let moves = Rc::new(RefCell::new(Vec::new()));
    let mut h = Harness::new(list(&["a", "b", "c"], &moves));
    let target = h.tree.focusable_elements()[0];
    h.tree.set_keyboard_focus(target, true);
    for (key, code) in [
        (KeyboardKey::Character(" ".into()), Code::Space),
        (KeyboardKey::Named(NamedKey::ArrowDown), Code::ArrowDown),
        (KeyboardKey::Named(NamedKey::Escape), Code::Escape),
    ] {
        let _ = h
            .tree
            .dispatch_keyboard(Some(target), KeyboardEvent::key_down(key, code));
        h.run(ms(32));
    }
    h.run(ms(600));
    assert!(moves.borrow().is_empty());
    assert!(
        h.transforms()
            .iter()
            .all(|t| t.translation_offset().y.abs() < 0.5)
    );
}

#[test]
fn kanban_moves_across_lists() {
    let moves: Rc<RefCell<Vec<Move<&'static str, u8>>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = moves.clone();
    let board =
        ReorderGroup::<&'static str, u8>::new().on_move(move |mv| sink.borrow_mut().push(mv));
    let card = |key: &'static str| -> Widget {
        Widget::box_(Size::new(100., 50.), Color::WHITE)
            .with_key(key)
            .reorder_item(key)
            .into()
    };
    let column = |id: u8, keys: &[&'static str]| -> Widget {
        Column::new(keys.iter().map(|key| card(key)))
            .reorderable_in(&board, id)
            .with_key(id as u64)
    };
    let mut h = Harness::new(
        Directionality::new(
            TextDirection::Ltr,
            Row::new([column(0, &["a", "b"]), column(1, &["c", "d"])]),
        )
        .into(),
    );
    // Columns are 100 wide side by side: drag a into the second column,
    // past c's center.
    h.drag(Offset::new(50., 25.), Offset::new(150., 40.));
    h.run(ms(300));
    h.release(Offset::new(150., 40.));
    let committed = moves.borrow().clone();
    assert_eq!(committed.len(), 1, "{committed:?}");
    assert_eq!(committed[0].from_list, 0);
    assert_eq!(committed[0].to_list, 1);
    assert_eq!(committed[0].to, 1);
}

#[test]
fn move_apply_reorders_vectors() {
    let mut items = vec!["a", "b", "c", "d"];
    Move {
        key: "a",
        from: 0,
        to: 2,
        from_list: (),
        to_list: (),
    }
    .apply(&mut items, |item| *item);
    assert_eq!(items, ["b", "c", "a", "d"]);
    let (mut left, mut right) = (vec!["a", "b"], vec!["c"]);
    Move {
        key: "b",
        from: 1,
        to: 0,
        from_list: 0,
        to_list: 1,
    }
    .apply_between(&mut left, &mut right, |item| *item);
    assert_eq!((left, right), (vec!["a"], vec!["b", "c"]));
}

type Moves = Rc<RefCell<Vec<Move<&'static str>>>>;

fn list_with(
    order: &[&'static str],
    moves: &Moves,
    item: impl Fn(&'static str) -> Widget,
    options: impl FnOnce(Reorderable<&'static str>) -> Reorderable<&'static str>,
) -> Widget {
    let sink = moves.clone();
    let list = Column::new(order.iter().map(|key| item(key)))
        .reorderable(move |mv| sink.borrow_mut().push(mv));
    Directionality::new(TextDirection::Ltr, options(list)).into()
}

#[test]
fn live_mode_reports_each_slot_change_during_the_drag() {
    let moves = Moves::default();
    let mut h = Harness::new(list_with(&["a", "b", "c"], &moves, row, |r| r.live(true)));
    h.drag(Offset::new(50., 25.), Offset::new(50., 85.));
    assert_eq!(moves.borrow().len(), 1, "reported before the drop");
    assert_eq!((moves.borrow()[0].from, moves.borrow()[0].to), (0, 1));
    h.release(Offset::new(50., 85.));
    assert_eq!(moves.borrow().len(), 1, "the drop adds nothing");
}

#[test]
fn pointer_within_uses_the_pointer_not_the_item_center() {
    // Grab a near its bottom and move 40 px: the item's center (65) is
    // before b's center (75), the pointer (85) is past it.
    for (collision, expected) in [(Collision::ClosestCenter, 0), (Collision::PointerWithin, 1)] {
        let moves = Moves::default();
        let mut h = Harness::new(list_with(&["a", "b", "c"], &moves, row, |r| {
            r.collision(collision)
        }));
        h.drag(Offset::new(50., 45.), Offset::new(50., 85.));
        h.release(Offset::new(50., 85.));
        let moved = moves.borrow().first().map_or(0, |mv| mv.to);
        assert_eq!(moved, expected, "{collision:?}");
    }
}

#[test]
fn axis_lock_keeps_the_drag_on_the_list_axis() {
    let moves = Moves::default();
    let mut h = Harness::new(list_with(
        &["a", "b", "c"],
        &moves,
        row,
        Reorderable::axis_lock,
    ));
    h.drag(Offset::new(50., 25.), Offset::new(110., 85.));
    let dragged = h.transforms()[0].translation_offset();
    assert!(dragged.x.abs() < 5., "no cross-axis motion: {dragged:?}");
    assert!(dragged.y > 50., "follows along the axis: {dragged:?}");
}

#[test]
fn a_handle_item_drags_only_from_its_handle() {
    let moves = Moves::default();
    let item = |key: &'static str| -> Widget {
        Widget::from(Row::new([
            Widget::box_(Size::new(20., 50.), Color::BLACK).reorder_handle(),
            Widget::box_(Size::new(80., 50.), Color::WHITE),
        ]))
        .with_key(key)
        .reorder_item(key)
        .with_handle()
        .into()
    };
    let mut h = Harness::new(list_with(&["a", "b", "c"], &moves, item, |r| r));
    h.drag(Offset::new(60., 25.), Offset::new(60., 85.));
    h.release(Offset::new(60., 85.));
    h.run(ms(600));
    assert!(moves.borrow().is_empty(), "the body does not start a drag");
    h.drag(Offset::new(10., 25.), Offset::new(10., 85.));
    h.release(Offset::new(10., 85.));
    assert_eq!(moves.borrow().len(), 1, "the handle does");
}

#[test]
fn a_ghost_placeholder_marks_the_target_slot() {
    let moves = Moves::default();
    let item = |key: &'static str| -> Widget {
        Widget::box_(Size::new(100., 50.), Color::WHITE)
            .with_key(key)
            .reorder_item(key)
            .placeholder(Placeholder::Ghost(0.4))
            .into()
    };
    let mut h = Harness::new(list_with(&["a", "b", "c"], &moves, item, |r| r));
    h.drag(Offset::new(50., 25.), Offset::new(50., 85.));
    h.run(ms(400));
    // Each item has its ghost transform first, then its own.
    let ys: Vec<f32> = h
        .transforms()
        .iter()
        .map(|t| t.translation_offset().y)
        .collect();
    assert!(
        (ys[0] - 50.).abs() < 1.,
        "a's ghost sits in b's old slot: {ys:?}"
    );
    assert!((ys[3] + 50.).abs() < 1., "b shifted up: {ys:?}");
}

#[test]
fn touch_needs_a_long_press_to_lift() {
    let moves = Moves::default();
    let mut h = Harness::new(list_with(&["a", "b", "c"], &moves, row, |r| r));
    let touch = |h: &mut Harness, at: Offset, phase: PointerPhase| {
        h.pointer_of(PointerDeviceKind::Touch, at, phase);
        h.frame();
    };
    // A quick swipe is left to scrolling.
    touch(&mut h, Offset::new(50., 25.), PointerPhase::Down);
    for y in [40., 60., 85.] {
        h.now += ms(16);
        touch(&mut h, Offset::new(50., y), PointerPhase::Move);
    }
    touch(&mut h, Offset::new(50., 85.), PointerPhase::Up);
    h.run(ms(600));
    assert!(moves.borrow().is_empty(), "a swipe does not reorder");
    // A long press lifts, then the finger drags.
    touch(&mut h, Offset::new(50., 25.), PointerPhase::Down);
    h.now += ms(700);
    touch(&mut h, Offset::new(50., 25.), PointerPhase::Move);
    for y in [40., 60., 85.] {
        h.now += ms(16);
        touch(&mut h, Offset::new(50., y), PointerPhase::Move);
    }
    touch(&mut h, Offset::new(50., 85.), PointerPhase::Up);
    assert_eq!(moves.borrow().len(), 1, "{:?}", moves.borrow());
}

#[test]
fn a_keyboard_drop_is_announced_to_screen_readers() {
    let moves = Moves::default();
    let item = |key: &'static str| -> Widget {
        Widget::box_(Size::new(100., 50.), Color::WHITE)
            .with_key(key)
            .reorder_item(key)
            .label(format!("Card {key}"))
            .into()
    };
    let mut h = Harness::new(list_with(&["a", "b", "c"], &moves, item, |r| r));
    let target = h.tree.focusable_elements()[0];
    h.tree.set_keyboard_focus(target, true);
    for (key, code) in [
        (KeyboardKey::Character(" ".into()), Code::Space),
        (KeyboardKey::Named(NamedKey::ArrowDown), Code::ArrowDown),
        (KeyboardKey::Character(" ".into()), Code::Space),
    ] {
        let _ = h
            .tree
            .dispatch_keyboard(Some(target), KeyboardEvent::key_down(key, code));
        h.run(ms(32));
    }
    h.tree.update_semantics();
    let dump = h.tree.semantics_debug_dump();
    assert!(dump.contains("Moved Card a to position 2 of 3"), "{dump}");
    assert!(dump.contains("Card b"), "items carry their labels: {dump}");
}

#[test]
fn dragging_near_a_scroll_edge_scrolls() {
    let moves = Moves::default();
    let controller = ScrollController::new();
    let sink = moves.clone();
    let list = Column::new(["a", "b", "c", "d", "e", "f"].into_iter().map(row))
        .reorderable(move |mv| sink.borrow_mut().push(mv));
    let view = SizedBox::new()
        .width(100.)
        .height(120.)
        .child(SingleChildScrollView::new(list).controller(controller.clone()));
    let mut h = Harness::new(Directionality::new(TextDirection::Ltr, view).into());
    // Hold a near the viewport's bottom edge (120).
    h.drag(Offset::new(50., 25.), Offset::new(50., 110.));
    for _ in 0..5 {
        h.now += ms(16);
        h.pointer(Offset::new(50., 112.), PointerPhase::Move);
        h.frame();
    }
    assert!(
        controller.offset() > 5.,
        "scrolled: {}",
        controller.offset()
    );
    h.release(Offset::new(50., 112.));
}
