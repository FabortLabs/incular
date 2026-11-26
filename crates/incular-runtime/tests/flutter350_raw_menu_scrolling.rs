use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::{Runtime, Signal};
use incular_scroll::ScrollController;
use incular_widgets::{
    Align, Alignment, GestureDetector, MenuController, RawMenuAnchor, SingleChildScrollView,
    SizedBox, Widget, internal::Key,
};

const PANEL: Color = Color::rgba(21, 32, 83, 255);

fn frame(runtime: &mut Runtime, now: Instant) -> DisplayList {
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), now)
        .unwrap()
        .0
}

fn visible(list: &DisplayList) -> bool {
    list.commands()
        .iter()
        .any(|command| matches!(command, PaintCommand::Rect { color, .. } if *color == PANEL))
}

#[test]
fn an_out_of_range_ancestor_jump_closes_only_an_open_menu() {
    let scroll = ScrollController::new();
    let menu = MenuController::new();
    let requests = Signal::new(0usize);
    let key = Key::from("anchor");
    let anchor = RawMenuAnchor::new(menu.clone(), |_, _| {
        Widget::box_(Size::new(60., 40.), PANEL)
    })
    .child(
        Widget::from(
            GestureDetector::new(Widget::box_(Size::new(30., 20.), Color::WHITE)).on_tap({
                let menu = menu.clone();
                move || menu.open(None)
            }),
        )
        .with_key(key.clone()),
    )
    .on_close_requested({
        let requests = requests.clone();
        move |hide| {
            requests.set(requests.get() + 1);
            hide();
        }
    });
    let scene = SingleChildScrollView::new(
        SizedBox::new()
            .height(1000.)
            .child(Align::new(Alignment::CENTER, anchor)),
    )
    .controller(scroll.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert_eq!(scroll.max_offset(), 400.);

    assert!(scroll.jump_to(10.));
    frame(&mut runtime, now + Duration::from_millis(1));
    assert_eq!(requests.get(), 0);

    let anchor = runtime.tree().element_with_key(&key).unwrap();
    let bounds = runtime.tree().element_bounds(anchor).unwrap();
    let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
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
                position,
            },
            now + Duration::from_millis(1),
        );
    }
    assert!(visible(&frame(
        &mut runtime,
        now + Duration::from_millis(2)
    )));
    assert_eq!(requests.get(), 0);

    assert!(scroll.jump_to(500.));
    let _ = frame(&mut runtime, now + Duration::from_millis(3));
    assert_eq!(requests.get(), 1);
    assert!(!menu.is_open());
    assert!(!visible(&frame(
        &mut runtime,
        now + Duration::from_millis(4)
    )));
    assert!(runtime.tree().element_with_key(&key).is_some());

    assert!(scroll.jump_to(5.));
    frame(&mut runtime, now + Duration::from_millis(5));
    assert!(runtime.tree().element_with_key(&key).is_some());
    assert_eq!(requests.get(), 1);
}

#[test]
fn only_the_nearest_activity_start_and_end_request_a_deferred_hide() {
    let outer = ScrollController::new();
    let inner = ScrollController::new();
    let menu = MenuController::new();
    let callbacks = Rc::new(RefCell::new(Vec::<Rc<dyn Fn()>>::new()));
    let anchor = RawMenuAnchor::new(menu.clone(), |_, _| {
        Widget::box_(Size::new(60., 40.), PANEL)
    })
    .child(Widget::box_(Size::new(30., 20.), Color::WHITE))
    .on_close_requested({
        let callbacks = callbacks.clone();
        move |hide| callbacks.borrow_mut().push(hide)
    });
    let inside = SingleChildScrollView::new(SizedBox::new().height(900.).child(anchor))
        .controller(inner.clone());
    let scene = SingleChildScrollView::new(SizedBox::new().height(1200.).child(Align::new(
        Alignment::TOP_LEFT,
        SizedBox::new().height(250.).width(300.).child(inside),
    )))
    .controller(outer.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    assert_eq!(inner.max_offset(), 650.);
    menu.open(None);
    assert!(visible(&frame(
        &mut runtime,
        now + Duration::from_millis(1)
    )));

    assert!(outer.begin_activity() && outer.end_activity());
    assert!(outer.jump_to(30.) && inner.jump_to(40.));
    frame(&mut runtime, now + Duration::from_millis(2));
    assert!(menu.is_open());
    assert!(callbacks.borrow().is_empty());

    assert!(inner.animate_to(100., Duration::from_millis(100), Curve::Linear));
    frame(&mut runtime, now + Duration::from_millis(3));
    assert_eq!(callbacks.borrow().len(), 1);
    frame(&mut runtime, now + Duration::from_millis(53));
    assert_eq!(callbacks.borrow().len(), 1);
    frame(&mut runtime, now + Duration::from_millis(104));
    assert_eq!(callbacks.borrow().len(), 2);
    assert!(menu.is_open());

    let hide = callbacks.borrow().last().unwrap().clone();
    hide();
    assert!(!visible(&frame(
        &mut runtime,
        now + Duration::from_millis(105)
    )));
    assert!(inner.jump_to(700.));
    frame(&mut runtime, now + Duration::from_millis(106));
    assert_eq!(callbacks.borrow().len(), 2);
}

#[test]
fn mounting_replacing_and_disposing_the_nearest_position_manage_its_listener() {
    let old = ScrollController::new();
    let replacement = ScrollController::new();
    let menu = MenuController::new();
    let requests = Signal::new(0usize);
    let scene = |position: &ScrollController, with_menu: bool| {
        let anchor: Widget = if with_menu {
            RawMenuAnchor::new(menu.clone(), |_, _| {
                Widget::box_(Size::new(60., 40.), PANEL)
            })
            .child(Widget::box_(Size::new(30., 20.), Color::WHITE))
            .on_close_requested({
                let requests = requests.clone();
                move |hide| {
                    requests.set(requests.get() + 1);
                    hide();
                }
            })
            .into()
        } else {
            SizedBox::shrink().into()
        };
        Widget::from(
            SingleChildScrollView::new(SizedBox::new().height(900.).child(anchor))
                .controller(position.clone()),
        )
    };
    let now = Instant::now();
    let mut runtime = Runtime::new(scene(&old, true)).unwrap();
    frame(&mut runtime, now);
    assert!(old.has_scrolling_listeners());
    assert!(!replacement.has_scrolling_listeners());

    let root = runtime.tree().root().unwrap();
    menu.open(None);
    frame(&mut runtime, now + Duration::from_millis(1));
    runtime
        .schedule_update(root, scene(&replacement, true))
        .unwrap();
    frame(&mut runtime, now + Duration::from_millis(2));
    assert!(
        menu.is_open(),
        "controller replacement retains the open menu"
    );
    assert!(!old.has_scrolling_listeners());
    assert!(replacement.has_scrolling_listeners());
    assert!(old.begin_activity() && old.end_activity());
    assert_eq!(requests.get(), 0);
    assert!(replacement.begin_activity() && replacement.end_activity());
    assert_eq!(requests.get(), 1);

    runtime
        .schedule_update(root, scene(&replacement, false))
        .unwrap();
    frame(&mut runtime, now + Duration::from_millis(3));
    assert!(!replacement.has_scrolling_listeners());
}

#[test]
fn scrolling_the_overlay_keeps_both_menus_open_until_the_outer_position_scrolls() {
    use incular_widgets::Column;
    let outer = ScrollController::new();
    let inner = ScrollController::new();
    let root = MenuController::new();
    let child = MenuController::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let button_key = Key::from("inner-menu-button");
    let submenu = RawMenuAnchor::new(child.clone(), |_, _| SizedBox::shrink().into())
        .child(
            Widget::from(
                GestureDetector::new(Widget::box_(Size::new(200., 300.), PANEL)).on_tap({
                    let child = child.clone();
                    move || child.open(None)
                }),
            )
            .with_key(button_key.clone()),
        )
        .on_close({
            let log = log.clone();
            move || log.borrow_mut().push("child")
        });
    let panel: Widget = SizedBox::new()
        .width(200.)
        .height(300.)
        .child(
            SingleChildScrollView::new(Column::new([
                Widget::from(submenu),
                Widget::box_(Size::new(200., 900.), Color::BLACK),
            ]))
            .controller(inner.clone()),
        )
        .into();
    let anchor = RawMenuAnchor::new(root.clone(), move |_, _| {
        Align::new(Alignment::TOP_LEFT, panel.clone()).into()
    })
    .child(Widget::box_(Size::new(30., 20.), Color::WHITE))
    .on_close({
        let log = log.clone();
        move || log.borrow_mut().push("root")
    });
    let scene = SingleChildScrollView::new(
        SizedBox::new()
            .height(700.)
            .child(Align::new(Alignment::TOP_LEFT, anchor)),
    )
    .controller(outer.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    root.open(None);
    frame(&mut runtime, now + Duration::from_millis(1));
    let button = runtime.tree().element_with_key(&button_key).unwrap();
    let bounds = runtime.tree().element_bounds(button).unwrap();
    let point = bounds.origin + Offset::new(20., 20.);
    for phase in [PointerPhase::Down, PointerPhase::Up, PointerPhase::Move] {
        let _ = runtime.handle_input_at(
            InputEvent::Pointer {
                phase,
                position: point,
            },
            now,
        );
    }
    frame(&mut runtime, now + Duration::from_millis(2));
    assert!(root.is_open() && child.is_open());
    let _ = runtime.handle_input_at(
        InputEvent::Scroll {
            delta: Offset::new(0., 30.),
        },
        now,
    );
    frame(&mut runtime, now + Duration::from_millis(3));
    assert_eq!((inner.offset(), outer.offset()), (30., 0.));
    assert!(root.is_open() && child.is_open());
    assert!(log.borrow().is_empty());

    assert!(outer.jump_to(700.));
    frame(&mut runtime, now + Duration::from_millis(4));
    assert!(!root.is_open() && !child.is_open());
    assert_eq!(*log.borrow(), ["child", "root"]);
}
