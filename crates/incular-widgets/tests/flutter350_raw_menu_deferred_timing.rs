use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Color, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Size, time::Instant};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_rendering::PaintCommand;
use incular_widgets::{
    Center, GestureDetector, MenuController, RawMenuAnchor, Widget,
    internal::{Key, WidgetTree},
};

type Hide = Rc<dyn Fn()>;
#[derive(Default)]
struct Clock {
    now: Cell<u64>,
    pending: RefCell<Vec<(u64, Hide)>>,
}
impl Clock {
    fn schedule(&self, delay: u64, hide: Hide) {
        self.pending
            .borrow_mut()
            .push((self.now.get() + delay, hide));
    }

    fn pump_to(&self, to: u64) {
        assert!(to >= self.now.get());
        loop {
            let next = {
                let mut pending = self.pending.borrow_mut();
                pending
                    .iter()
                    .enumerate()
                    .filter(|(_, (when, _))| *when <= to)
                    .min_by_key(|(_, (when, _))| *when)
                    .map(|(index, _)| index)
                    .map(|index| pending.remove(index))
            };
            let Some((when, hide)) = next else { break };
            self.now.set(when);
            hide();
        }
        self.now.set(to);
    }
}

const PANEL: Color = Color::rgba(83, 28, 14, 255);
fn anchor_button() -> Widget {
    Widget::box_(Size::new(30., 20.), Color::WHITE)
}

fn panel_visible(tree: &mut WidgetTree) -> bool {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.paint()
        .commands()
        .iter()
        .any(|command| matches!(command, PaintCommand::Rect { color, .. } if *color == PANEL))
}

fn tap_key(tree: &mut WidgetTree, key: Key) {
    let element = tree.element_with_key(&key).expect("live menu button");
    let bounds = tree.element_bounds(element).unwrap();
    let point = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let event = RawPointerEvent {
            buttons,
            button: Some(PRIMARY_POINTER_BUTTON),
            ..RawPointerEvent::new(
                0,
                1,
                PointerDeviceKind::Touch,
                0,
                point,
                phase,
                Instant::now(),
            )
        };
        let hit = tree.pointer_hit_test(point);
        let _ = tree.dispatch_raw_pointer_with_hit(1, event, &hit);
        let _ = tree.dispatch_pointer_gesture_raw_with_hit(1, event, &hit);
    }
}

#[test]
fn delayed_hide_not_the_request_removes_overlay_and_notifies_on_close() {
    let clock = Rc::new(Clock::default());
    let controller = MenuController::new();
    let requests = Rc::new(Cell::new(0));
    let closes = Rc::new(Cell::new(0));
    let menu = RawMenuAnchor::new(controller.clone(), |_, _| {
        Widget::box_(Size::new(300., 300.), PANEL)
    })
    .child(anchor_button())
    .on_close_requested({
        let (clock, requests) = (clock.clone(), requests.clone());
        move |hide| {
            requests.set(requests.get() + 1);
            clock.schedule(100, hide);
        }
    })
    .on_close({
        let closes = closes.clone();
        move || closes.set(closes.get() + 1)
    });
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(menu).into()).unwrap();
    assert!(!panel_visible(&mut tree));
    controller.open(None);
    assert!(panel_visible(&mut tree));
    controller.close();
    assert_eq!((requests.get(), closes.get()), (1, 0));
    assert!(panel_visible(&mut tree));
    clock.pump_to(99);
    assert!(panel_visible(&mut tree));
    assert_eq!(closes.get(), 0);
    clock.pump_to(101);
    assert!(!panel_visible(&mut tree));
    assert_eq!(closes.get(), 1);

    // Pinned RawMenuAnchor forwards requests even after hiding, but the
    // callback cannot close an already hidden overlay a second time.
    controller.close();
    assert_eq!(requests.get(), 2);
    clock.pump_to(201);
    assert!(!panel_visible(&mut tree));
    assert_eq!(closes.get(), 1);
}

#[test]
fn retained_four_deep_tree_requests_from_the_root_and_hides_descendants_first() {
    let clock = Rc::new(Clock::default());
    let requests = Rc::new(RefCell::new(Vec::new()));
    let closes = Rc::new(RefCell::new(Vec::new()));
    let controllers: Vec<_> = (0..4).map(|_| MenuController::new()).collect();
    let mut content = Widget::box_(Size::new(70., 50.), PANEL);
    for (index, delay) in [(0, 35), (1, 50), (2, 10), (3, 25)].into_iter().rev() {
        let controller = controllers[index].clone();
        let next = content;
        content = RawMenuAnchor::new(controller.clone(), move |_, _| next.clone())
            .child(
                Widget::from(
                    GestureDetector::new(anchor_button()).on_tap(move || controller.open(None)),
                )
                .with_key(Key::Value(index as u64)),
            )
            .on_close_requested({
                let (clock, requests) = (clock.clone(), requests.clone());
                move |hide| {
                    requests.borrow_mut().push(index);
                    clock.schedule(delay, hide);
                }
            })
            .on_close({
                let closes = closes.clone();
                move || closes.borrow_mut().push(index)
            })
            .into();
    }
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(content).into()).unwrap();
    assert!(!panel_visible(&mut tree));
    controllers[0].open(None);
    let _ = panel_visible(&mut tree);
    for index in 1..4 {
        tap_key(&mut tree, Key::Value(index));
        assert_eq!(panel_visible(&mut tree), index == 3);
    }
    assert!(controllers.iter().all(MenuController::is_open));

    // Match the pinned test: clear setup observations, without draining
    // any timers queued while the submenus opened at this same instant.
    requests.borrow_mut().clear();
    closes.borrow_mut().clear();
    controllers[0].close();
    assert_eq!(*requests.borrow(), [0, 1, 2, 3]);

    for (ms, expected) in [
        (9, &[][..]),
        (11, &[3, 2]),
        (34, &[3, 2]),
        (36, &[3, 2, 1, 0]),
        (56, &[3, 2, 1, 0]),
    ] {
        clock.pump_to(ms);
        assert_eq!(&*closes.borrow(), expected, "{ms}ms");
        assert_eq!(panel_visible(&mut tree), ms < 10, "{ms}ms");
    }
    assert!(controllers.iter().all(|controller| !controller.is_open()));
}

#[test]
fn open_requests_forward_optional_positions_but_only_show_notifies_open() {
    let controller = MenuController::new();
    let requests = Rc::new(RefCell::new(Vec::<Option<Offset>>::new()));
    let pending = Rc::new(RefCell::new(None::<Hide>));
    let opened = Rc::new(Cell::new(0));
    let menu = RawMenuAnchor::new(controller.clone(), |_, _| {
        Widget::box_(Size::new(40., 40.), PANEL)
    })
    .child(anchor_button())
    .on_open_requested({
        let (requests, pending) = (requests.clone(), pending.clone());
        move |position, show| {
            requests.borrow_mut().push(position);
            *pending.borrow_mut() = Some(show);
        }
    })
    .on_open({
        let opened = opened.clone();
        move || opened.set(opened.get() + 1)
    });
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(menu).into()).unwrap();
    assert!(!panel_visible(&mut tree));
    assert!(requests.borrow().is_empty());
    assert_eq!(opened.get(), 0);

    controller.open(None);
    assert!(!panel_visible(&mut tree));
    assert_eq!(*requests.borrow(), [None]);
    assert_eq!(opened.get(), 0);
    controller.close();
    assert_eq!(*requests.borrow(), [None]);

    let at = Offset::new(10., 15.);
    controller.open(Some(at));
    assert_eq!(*requests.borrow(), [None, Some(at)]);
    assert_eq!(opened.get(), 0);
    let show = pending.borrow().clone().unwrap();
    show();
    assert!(panel_visible(&mut tree));
    assert_eq!(opened.get(), 1);
    show();
    assert!(panel_visible(&mut tree));
    assert_eq!(opened.get(), 2);
    assert_eq!(*requests.borrow(), [None, Some(at)]);
}

#[test]
fn a_scheduled_show_places_a_live_panel_at_zero_or_the_requested_position() {
    use incular_widgets::{Positioned, Stack};
    let controller = MenuController::new();
    let clock = Rc::new(Clock::default());
    let panel = Key::from("scheduled-panel");
    let panel_key = panel.clone();
    let menu = RawMenuAnchor::new(controller.clone(), move |_, info| {
        let at = info.position.unwrap_or(Offset::ZERO);
        Stack::new([Widget::from(
            Positioned::new(Widget::box_(Size::new(300., 300.), PANEL).with_key(panel_key.clone()))
                .left(at.x)
                .top(at.y),
        )])
        .into()
    })
    .child(anchor_button())
    .on_open_requested({
        let clock = clock.clone();
        move |_, show| clock.schedule(100, show)
    });
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(menu).into()).unwrap();
    assert!(!panel_visible(&mut tree));
    controller.open(None);
    assert!(!panel_visible(&mut tree));
    clock.pump_to(101);
    assert!(panel_visible(&mut tree));
    let origin = |tree: &WidgetTree| {
        tree.element_bounds(tree.element_with_key(&panel).unwrap())
            .unwrap()
            .origin
    };
    assert_eq!(origin(&tree), Offset::ZERO);

    let at = Offset::new(50., 50.);
    controller.open(Some(at));
    assert!(panel_visible(&mut tree));
    assert_eq!(origin(&tree), Offset::ZERO);
    clock.pump_to(202);
    assert!(panel_visible(&mut tree));
    assert_eq!(origin(&tree), at);
}

#[test]
fn a_saved_show_cannot_reopen_a_disposed_anchor() {
    let controller = MenuController::new();
    let pending = Rc::new(RefCell::new(None::<Hide>));
    let opened = Rc::new(Cell::new(0));
    let menu = RawMenuAnchor::new(controller.clone(), |_, _| {
        Widget::box_(Size::new(40., 40.), PANEL)
    })
    .child(anchor_button())
    .on_open_requested({
        let pending = pending.clone();
        move |_, show| *pending.borrow_mut() = Some(show)
    })
    .on_open({
        let opened = opened.clone();
        move || opened.set(opened.get() + 1)
    });
    let mut tree = WidgetTree::new();
    let root = tree.mount(Center::new(menu).into()).unwrap();
    assert!(!panel_visible(&mut tree));
    controller.open(None);
    let show = pending.borrow().clone().unwrap();
    show();
    assert!(panel_visible(&mut tree));
    assert_eq!(opened.get(), 1);
    controller.close();
    assert!(!panel_visible(&mut tree));
    tree.update(root, Center::new(anchor_button()).into())
        .unwrap();
    assert!(!panel_visible(&mut tree));
    show();
    assert!(!panel_visible(&mut tree));
    assert_eq!(opened.get(), 1);
    assert!(!controller.is_open());
}
