use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, Transform as Affine, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Column, GestureDetector, MenuController, Positioned, RawMenuAnchor, RawMenuAnchorGroup,
    RawMenuOverlayInfo, SizedBox, Stack, Transform, Widget, internal::Key,
};
use std::{cell::RefCell, rc::Rc};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const ANCHOR_SIZE: Size = Size {
    width: 100.,
    height: 20.,
};
const PANEL: Color = Color::rgba(10, 148, 21, 255);

fn scene(
    controller: &MenuController,
    log: &Rc<RefCell<Vec<RawMenuOverlayInfo>>>,
    tagged_panel: bool,
) -> Widget {
    let log = log.clone();
    RawMenuAnchor::new(controller.clone(), move |_, info| {
        log.borrow_mut().push(info.clone());
        if tagged_panel {
            Stack::new([Widget::from(
                Positioned::new(
                    Widget::box_(Size::new(200., 200.), PANEL).with_key(Key::from("panel")),
                )
                .left(info.anchor_rect.origin.x)
                .top(info.anchor_rect.origin.y),
            )])
            .into()
        } else {
            SizedBox::shrink().into()
        }
    })
    .child(
        Widget::from(
            GestureDetector::new(Widget::box_(ANCHOR_SIZE, Color::WHITE)).on_tap({
                let controller = controller.clone();
                move || controller.open(None)
            }),
        )
        .with_key(Key::from("anchor-button")),
    )
    .into()
}

fn frame(runtime: &mut Runtime) {
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
}

fn bounds(runtime: &Runtime, key: &str) -> Rect {
    runtime
        .tree()
        .element_bounds(runtime.tree().element_with_key(&Key::from(key)).unwrap())
        .unwrap()
}

fn open_with_tap(runtime: &mut Runtime) {
    let rect = bounds(runtime, "anchor-button");
    let point = rect.origin + Offset::new(rect.size.width / 2., rect.size.height / 2.);
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
                position: point,
            },
            Instant::now(),
        );
    }
    frame(runtime);
}

#[test]
fn overlay_layout_info_can_place_content_at_the_real_anchor_corner() {
    let menu = MenuController::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = Runtime::new(Center::new(scene(&menu, &log, true)).into()).unwrap();
    frame(&mut runtime);
    assert!(log.borrow().is_empty());
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("panel"))
            .is_none()
    );

    open_with_tap(&mut runtime);
    assert!(menu.is_open());
    let anchor = bounds(&runtime, "anchor-button");
    let log = log.borrow();
    let info = log.last().unwrap();
    assert_eq!(info.anchor_rect, anchor);
    assert_eq!(info.overlay_size, VIEW);
    assert_eq!(
        bounds(&runtime, "panel"),
        Rect::from_origin_size(anchor.origin, Size::new(200., 200.))
    );
}

#[test]
fn overlay_info_transforms_the_anchor_before_publishing_it() {
    let menu = MenuController::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let transform = Affine::translation(Offset::new(-50., 50.)).then(Affine::scale(1.2));
    let mut runtime =
        Runtime::new(Center::new(Transform::new(transform, scene(&menu, &log, false))).into())
            .unwrap();
    frame(&mut runtime);
    menu.open(None);
    frame(&mut runtime);
    let physical = bounds(&runtime, "anchor-button");
    let actual = log.borrow().last().unwrap().anchor_rect;
    assert_eq!(actual, physical);
    for (observed, expected) in [
        (actual.origin.x, 300.),
        (actual.origin.y, 340.),
        (actual.size.width, 120.),
        (actual.size.height, 24.),
    ] {
        assert!((observed - expected).abs() < 0.0002, "{actual:?}");
    }
}

#[test]
fn moving_a_retained_anchor_updates_its_open_overlay_geometry() {
    let menu = MenuController::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let anchor = scene(&menu, &log, false).with_key(Key::from("retained-menu"));
    let path = |extra: bool| {
        let mut children = Vec::new();
        if extra {
            children.push(Widget::from(SizedBox::new().width(100.).height(100.)));
        }
        children.push(anchor.clone());
        Widget::from(Center::new(Column::new(children)))
    };
    let mut runtime = Runtime::new(path(true)).unwrap();
    frame(&mut runtime);
    open_with_tap(&mut runtime);
    let before = bounds(&runtime, "anchor-button");
    assert_eq!(log.borrow().last().unwrap().anchor_rect, before);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, path(false)).unwrap();
    frame(&mut runtime);
    let after = bounds(&runtime, "anchor-button");
    assert_eq!(log.borrow().last().unwrap().anchor_rect, after);
    assert_eq!(before.origin.y - after.origin.y, 100.);
    assert!(menu.is_open());
}

#[test]
fn opening_a_zero_area_menu_remains_safe() {
    let menu = MenuController::new();
    let anchor = RawMenuAnchor::new(menu.clone(), |_, _| {
        Widget::box_(Size::new(30., 20.), PANEL)
    });
    let mut runtime = Runtime::new(Center::new(anchor).into()).unwrap();
    let zero = Constraints::tight(Size::ZERO);
    runtime.run_frame(zero).unwrap();
    assert_eq!(
        runtime
            .tree()
            .element_bounds(runtime.tree().root().unwrap())
            .unwrap()
            .size,
        Size::ZERO
    );
    menu.open(None);
    runtime.run_frame(zero).unwrap();
    assert!(menu.is_open());
}

#[test]
fn a_zero_area_group_cannot_open_itself_and_remains_safe() {
    let menu = MenuController::new();
    let group = RawMenuAnchorGroup::new(menu.clone(), Widget::box_(Size::new(30., 20.), PANEL));
    let mut runtime = Runtime::new(Center::new(group).into()).unwrap();
    let zero = Constraints::tight(Size::ZERO);
    runtime.run_frame(zero).unwrap();
    assert_eq!(
        runtime
            .tree()
            .element_bounds(runtime.tree().root().unwrap())
            .unwrap()
            .size,
        Size::ZERO
    );
    menu.open(None);
    runtime.run_frame(zero).unwrap();
    assert!(!menu.is_open());
}

#[test]
fn nested_menu_surface_uses_the_roots_overlay_choice_instead_of_its_own() {
    for root_uses_root_overlay in [false, true] {
        let root = MenuController::new();
        let child = MenuController::new();
        let content: Widget = RawMenuAnchor::new(child.clone(), |_, _| {
            Widget::box_(Size::new(70., 30.), PANEL)
        })
        .use_root_overlay(!root_uses_root_overlay)
        .child(Widget::box_(ANCHOR_SIZE, Color::WHITE))
        .into();
        let main = RawMenuAnchor::new(root.clone(), move |_, _| content.clone())
            .use_root_overlay(root_uses_root_overlay)
            .child(Widget::box_(ANCHOR_SIZE, Color::WHITE));
        let mut runtime = Runtime::new(Center::new(main).into()).unwrap();
        frame(&mut runtime);
        root.open(None);
        frame(&mut runtime);
        let first = runtime.tree().transient_surfaces()[0].id;
        child.open(None);
        frame(&mut runtime);
        let surfaces = runtime.tree().transient_surfaces();
        assert_eq!(surfaces.len(), 2);
        let nested = surfaces.iter().find(|entry| entry.id != first).unwrap();
        assert_eq!(
            nested.parent,
            (!root_uses_root_overlay).then_some(first),
            "root uses root overlay: {root_uses_root_overlay}"
        );
    }
}

#[test]
fn an_open_nested_surface_retargets_when_its_root_changes_overlay_choice() {
    let root = MenuController::new();
    let child = MenuController::new();
    let panel: Widget = RawMenuAnchor::new(child.clone(), |_, _| {
        Widget::box_(Size::new(50., 20.), PANEL)
    })
    .child(Widget::box_(ANCHOR_SIZE, Color::WHITE))
    .into();
    let scene = |use_root_overlay| {
        let panel = panel.clone();
        Widget::from(Center::new(
            RawMenuAnchor::new(root.clone(), move |_, _| panel.clone())
                .use_root_overlay(use_root_overlay)
                .child(Widget::box_(ANCHOR_SIZE, Color::WHITE)),
        ))
    };
    let mut runtime = Runtime::new(scene(false)).unwrap();
    frame(&mut runtime);
    root.open(None);
    frame(&mut runtime);
    child.open(None);
    frame(&mut runtime);
    let initial = runtime.tree().transient_surfaces();
    assert_eq!(initial.len(), 2);
    assert_eq!(
        initial
            .iter()
            .filter(|entry| entry.parent.is_some())
            .count(),
        1
    );

    let widget_root = runtime.tree().root().unwrap();
    runtime.schedule_update(widget_root, scene(true)).unwrap();
    frame(&mut runtime);
    let updated = runtime.tree().transient_surfaces();
    assert!(root.is_open() && child.is_open());
    assert_eq!(updated.len(), 2);
    assert!(updated.iter().all(|entry| entry.parent.is_none()));
}

#[test]
fn transformed_anchors_do_not_transform_their_overlay_controls_again() {
    use incular_rendering::PaintCommand;
    use std::cell::Cell;
    let menu = MenuController::new();
    let hits = Rc::new(Cell::new(0));
    let published = Rc::new(Cell::new(None));
    let panel_key = Key::from("absolute-preview-panel");
    let expected = Rect::from_origin_size(Offset::new(380., 405.), Size::new(200., 40.));
    let anchor = RawMenuAnchor::new(menu.clone(), {
        let hits = hits.clone();
        let key = panel_key.clone();
        let published = published.clone();
        move |_, info| {
            published.set(Some(info.anchor_rect));
            let target = info.anchor_rect.origin + Offset::new(80., 65.);
            let hits = hits.clone();
            let item = Widget::from(
                GestureDetector::new(Widget::box_(Size::new(200., 40.), PANEL))
                    .on_tap(move || hits.set(hits.get() + 1)),
            )
            .with_key(key.clone());
            Stack::new([Widget::from(
                Positioned::new(item).left(target.x).top(target.y),
            )])
            .into()
        }
    })
    .child(
        Widget::from(SizedBox::new().width(100.).height(20.))
            .with_key(Key::from("transformed-anchor")),
    );
    let transform = Affine::translation(Offset::new(-50., 50.)).then(Affine::scale(1.2));
    let mut runtime = Runtime::new(Center::new(Transform::new(transform, anchor)).into()).unwrap();
    frame(&mut runtime);
    menu.open(None);
    let list = runtime.run_frame(Constraints::tight(VIEW)).unwrap().0;
    let mut matrices = vec![Affine::IDENTITY];
    let mut panels = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                matrices.push(matrices.last().unwrap().then(*transform));
            }
            PaintCommand::PopTransform => {
                matrices.pop();
            }
            PaintCommand::Rect { rect, color } if *color == PANEL => {
                panels.push(matrices.last().unwrap().transform_rect_bbox(*rect));
            }
            _ => {}
        }
    }
    let observed = bounds(&runtime, "absolute-preview-panel");
    assert_eq!(panels.len(), 1);
    let anchor = bounds(&runtime, "transformed-anchor");
    assert_eq!(published.get(), Some(anchor));
    assert_eq!(anchor.origin, Offset::new(300., 340.));
    for actual in [observed, panels[0]] {
        assert!(
            (actual.origin.x - expected.origin.x).abs()
                + (actual.origin.y - expected.origin.y).abs()
                < 0.0002,
            "{actual:?}"
        );
        assert!(
            (actual.size.width - expected.size.width).abs() < 0.0002,
            "{actual:?}"
        );
        assert!(
            (actual.size.height - expected.size.height).abs() < 0.0002,
            "{actual:?}"
        );
    }
    let position = expected.origin + Offset::new(195., 5.);
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
    assert_eq!(hits.get(), 1);
}

#[test]
fn a_group_panel_is_a_transparent_host_for_its_padded_content_geometry() {
    use incular_config::EdgeInsets;
    use incular_widgets::{
        Align, Alignment, Directionality, MainAxisSize, Padding, Row, TextDirection,
    };
    let panel = Widget::from(Padding::new(
        EdgeInsets::all(8.),
        Row::new([
            Widget::box_(Size::new(100., 100.), Color::WHITE),
            Widget::box_(Size::new(100., 100.), PANEL),
        ])
        .main_axis_size(MainAxisSize::Min),
    ))
    .with_key(Key::from("group-panel"));
    let group = RawMenuAnchorGroup::new(MenuController::new(), panel);
    let mut runtime = Runtime::new(
        Directionality::new(TextDirection::Ltr, Align::new(Alignment::TOP_LEFT, group)).into(),
    )
    .unwrap();
    frame(&mut runtime);
    assert_eq!(
        bounds(&runtime, "group-panel"),
        Rect::from_origin_size(Offset::ZERO, Size::new(216., 116.)),
    );
}
