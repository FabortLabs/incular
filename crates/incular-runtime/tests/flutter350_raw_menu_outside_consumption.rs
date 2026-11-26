use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Column, GestureDetector, MenuController, Positioned, RawMenuAnchor, RawMenuAnchorGroup,
    RawMenuOverlayInfo, SizedBox, Stack, TapRegion, TapRegionSurface, Widget, internal::Key,
};

type Log = Rc<RefCell<Vec<&'static str>>>;

fn button(key: &'static str, on_tap: impl Fn() + 'static) -> Widget {
    Widget::from(
        GestureDetector::new(Widget::box_(Size::new(100., 30.), Color::WHITE)).on_tap(on_tap),
    )
    .with_key(Key::from(key))
}

fn position(info: &RawMenuOverlayInfo, child: Widget, offset: Offset) -> Widget {
    let p = info.anchor_rect.origin + offset;
    Stack::new([Widget::from(Positioned::new(child).left(p.x).top(p.y))]).into()
}

fn panel(info: &RawMenuOverlayInfo, child: Widget) -> Widget {
    position(
        info,
        TapRegion::new(child)
            .group_id(info.tap_region_group_id.clone())
            .into(),
        Offset::new(0., info.anchor_rect.size.height),
    )
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn tap(runtime: &mut Runtime, key: &'static str) {
    let element = runtime
        .tree()
        .element_with_key(&Key::from(key))
        .expect("mounted button");
    let bounds = runtime.tree().element_bounds(element).unwrap();
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
            Instant::now(),
        );
    }
    frame(runtime);
}

fn entry(
    label: &'static str,
    controller: MenuController,
    content: Widget,
    consume: bool,
    opened: Log,
    closed: Log,
    selected: Log,
) -> Widget {
    RawMenuAnchor::new(controller.clone(), move |_, info| {
        panel(info, content.clone())
    })
    .consume_outside_taps(consume)
    .child(button(label, move || {
        selected.borrow_mut().push(label);
        controller.open(None);
    }))
    .on_open(move || opened.borrow_mut().push(label))
    .on_close(move || closed.borrow_mut().push(label))
    .into()
}

fn outside_activation(consume: bool) {
    let group = MenuController::new();
    let root = MenuController::new();
    let nested = MenuController::new();
    let selected = Log::default();
    let opened = Log::default();
    let closed = Log::default();
    let submenu = entry(
        "submenu",
        nested.clone(),
        SizedBox::new().width(40.).height(20.).into(),
        consume,
        opened.clone(),
        closed.clone(),
        selected.clone(),
    );
    let anchor = entry(
        "root",
        root.clone(),
        submenu,
        consume,
        opened.clone(),
        closed.clone(),
        selected.clone(),
    );
    let outside = button("outside", {
        let selected = selected.clone();
        move || selected.borrow_mut().push("outside")
    });
    let scene = Column::new([
        outside,
        RawMenuAnchorGroup::new(group.clone(), anchor).into(),
    ]);
    let mut runtime = Runtime::new(TapRegionSurface::new(Center::new(scene)).into()).unwrap();
    frame(&mut runtime);
    tap(&mut runtime, "outside");
    assert_eq!(*selected.borrow(), ["outside"]);
    assert!(opened.borrow().is_empty() && closed.borrow().is_empty());
    selected.borrow_mut().clear();

    tap(&mut runtime, "root");
    tap(&mut runtime, "submenu");
    assert_eq!(*opened.borrow(), ["root", "submenu"]);
    assert_eq!(*selected.borrow(), ["root", "submenu"]);
    assert!(closed.borrow().is_empty());
    assert!(group.is_open() && root.is_open() && nested.is_open());
    opened.borrow_mut().clear();
    selected.borrow_mut().clear();
    tap(&mut runtime, "outside");
    assert_eq!(*closed.borrow(), ["submenu", "root"]);
    assert!(opened.borrow().is_empty());
    assert!(!group.is_open() && !root.is_open() && !nested.is_open());
    let expected = if consume { &[][..] } else { &["outside"][..] };
    assert_eq!(&*selected.borrow(), expected);
}

#[test]
fn open_menus_can_consume_outside_activation_but_closed_menus_do_not() {
    outside_activation(true);
}

#[test]
fn unconsumed_outside_activation_still_closes_the_whole_tree() {
    outside_activation(false);
}

#[test]
fn passing_the_overlay_group_id_changes_how_an_anchor_tap_is_classified() {
    use std::cell::Cell;
    let menu = MenuController::new();
    let inside = Rc::new(Cell::new(None));
    let anchor = RawMenuAnchor::new(menu.clone(), {
        let inside = inside.clone();
        move |_, info| {
            let mut region = TapRegion::new(
                Widget::box_(Size::new(100., 30.), Color::BLACK).with_key(Key::from("panel")),
            )
            .on_tap_inside({
                let inside = inside.clone();
                move |_| inside.set(Some(true))
            })
            .on_tap_outside({
                let inside = inside.clone();
                move |_| inside.set(Some(false))
            });
            if !inside.get().unwrap_or(false) {
                region = region.group_id(info.tap_region_group_id.clone());
            }
            position(info, region.into(), Offset::new(200., 200.))
        }
    })
    .child(button("anchor", {
        let menu = menu.clone();
        move || {
            if menu.is_open() {
                menu.close();
            } else {
                menu.open(None);
            }
        }
    }));
    let mut runtime = Runtime::new(TapRegionSurface::new(Center::new(anchor)).into()).unwrap();
    frame(&mut runtime);
    let panel_is_mounted = |runtime: &Runtime| {
        runtime
            .tree()
            .element_with_key(&Key::from("panel"))
            .is_some()
    };

    tap(&mut runtime, "anchor");
    assert!(panel_is_mounted(&runtime));
    assert_eq!(inside.get(), None);
    tap(&mut runtime, "anchor");
    assert_eq!(inside.get(), Some(true));
    assert!(!panel_is_mounted(&runtime));
    tap(&mut runtime, "anchor");
    assert!(panel_is_mounted(&runtime));
    assert_eq!(inside.get(), Some(true));
    tap(&mut runtime, "anchor");
    assert_eq!(inside.get(), Some(false));
}
