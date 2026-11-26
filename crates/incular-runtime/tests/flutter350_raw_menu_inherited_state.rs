use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Center, Column, MenuController, RawMenuAnchor, RawMenuAnchorGroup, Widget,
};

type OpenLog = Rc<RefCell<Vec<Option<bool>>>>;

fn probe(log: &OpenLog) -> Widget {
    let log = log.clone();
    Builder::new(move |context| {
        log.borrow_mut()
            .push(MenuController::maybe_is_open_of(context));
        leaf()
    })
    .into()
}

fn leaf() -> Widget {
    Widget::box_(Size::new(80., 20.), Color::WHITE)
}

fn frame(runtime: &mut Runtime) {
    let _ = runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn open_observers_rebuild_only_when_their_nearest_menu_open_state_changes() {
    let group = MenuController::new();
    let root = MenuController::new();
    let nested = MenuController::new();
    let panel_log = OpenLog::default();
    let anchor_log = OpenLog::default();
    let overlay_log = OpenLog::default();
    let nested_menu = RawMenuAnchor::new(nested.clone(), |_, _| leaf()).child(leaf());
    let main_overlay: Widget = Column::new([probe(&overlay_log), nested_menu.into()]).into();
    let main_menu = RawMenuAnchor::new(root.clone(), move |_, _| main_overlay.clone())
        .child(probe(&anchor_log));
    let scene = RawMenuAnchorGroup::new(
        group.clone(),
        Column::new([probe(&panel_log), main_menu.into()]),
    );
    let outer_log = OpenLog::default();
    let outer_probe = probe(&outer_log);
    let mut runtime =
        Runtime::new(Center::new(Column::new([outer_probe, scene.into()])).into()).unwrap();
    frame(&mut runtime);
    assert_eq!(*outer_log.borrow(), [None]);
    assert_eq!(*panel_log.borrow(), [Some(false)]);
    assert_eq!(*anchor_log.borrow(), [Some(false)]);
    assert!(overlay_log.borrow().is_empty());

    root.open(None);
    frame(&mut runtime);
    assert_eq!(*panel_log.borrow(), [Some(false), Some(true)]);
    assert_eq!(*anchor_log.borrow(), [Some(false), Some(true)]);
    assert_eq!(*overlay_log.borrow(), [Some(true)]);

    nested.open(None);
    frame(&mut runtime);
    assert!(nested.is_open());
    assert_eq!(*panel_log.borrow(), [Some(false), Some(true)]);
    assert_eq!(*anchor_log.borrow(), [Some(false), Some(true)]);
    assert_eq!(*overlay_log.borrow(), [Some(true)]);

    root.close();
    frame(&mut runtime);
    assert_eq!(*outer_log.borrow(), [None]);
    assert_eq!(*panel_log.borrow(), [Some(false), Some(true), Some(false)]);
    assert_eq!(*anchor_log.borrow(), [Some(false), Some(true), Some(false)]);
    assert_eq!(*overlay_log.borrow(), [Some(true)]);
}

type ControllerLog = Rc<RefCell<Vec<Option<MenuController>>>>;
fn controller_probe(log: &ControllerLog) -> Widget {
    let log = log.clone();
    Builder::new(move |context| {
        log.borrow_mut().push(MenuController::maybe_of(context));
        leaf()
    })
    .into()
}

#[test]
fn controller_lookup_does_not_subscribe_but_rebuilt_widgets_find_the_replacement() {
    let watched = MenuController::new();
    let first = MenuController::new();
    let second = MenuController::new();
    let panel = ControllerLog::default();
    let anchor = ControllerLog::default();
    let overlay = ControllerLog::default();
    let build = |panel_controller: &MenuController, anchor_controller: &MenuController| {
        let overlay = controller_probe(&overlay);
        let menu = RawMenuAnchor::new(anchor_controller.clone(), move |_, _| overlay.clone())
            .child(controller_probe(&anchor));
        let group = RawMenuAnchorGroup::new(
            panel_controller.clone(),
            Column::new([controller_probe(&panel), menu.into()]),
        );
        Widget::from(Center::new(group))
    };
    let mut runtime = Runtime::new(build(&first, &second)).unwrap();
    frame(&mut runtime);
    assert!(panel.borrow().last().unwrap().as_ref() == Some(&first));
    assert!(anchor.borrow().last().unwrap().as_ref() == Some(&second));
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, build(&watched, &second))
        .unwrap();
    frame(&mut runtime);
    assert!(panel.borrow().last().unwrap().as_ref() == Some(&watched));
    assert!(anchor.borrow().last().unwrap().as_ref() == Some(&second));
    assert_eq!(
        (
            panel.borrow().len(),
            anchor.borrow().len(),
            overlay.borrow().len()
        ),
        (2, 2, 0)
    );

    anchor.borrow().last().unwrap().as_ref().unwrap().open(None);
    frame(&mut runtime);
    assert_eq!(
        (
            panel.borrow().len(),
            anchor.borrow().len(),
            overlay.borrow().len()
        ),
        (2, 2, 1)
    );
    assert!(overlay.borrow().last().unwrap().as_ref() == Some(&second));

    runtime
        .schedule_update(root, build(&first, &watched))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(
        (
            panel.borrow().len(),
            anchor.borrow().len(),
            overlay.borrow().len()
        ),
        (3, 3, 2)
    );
    assert!(panel.borrow().last().unwrap().as_ref() == Some(&first));
    assert!(anchor.borrow().last().unwrap().as_ref() == Some(&watched));
    assert!(overlay.borrow().last().unwrap().as_ref() == Some(&watched));
    assert!(!second.is_open());
    assert!(watched.is_open());
}

#[test]
fn the_anchor_builder_sees_its_own_controller_and_taps_toggle_its_visual_state() {
    use incular_core::{
        InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
        PointerSampleMetadata, time::Instant,
    };
    use incular_rendering::PaintCommand;
    use incular_widgets::{GestureDetector, Positioned, Stack, internal::Key};
    use std::cell::Cell;

    const OPEN_BUTTON: Color = Color::rgba(200, 0, 100, 255);
    const CLOSE_BUTTON: Color = Color::rgba(50, 50, 150, 255);
    let controller = MenuController::new();
    let opened = Rc::new(Cell::new(0));
    let closed = Rc::new(Cell::new(0));
    let button = Key::from("menu-toggle");
    let build_key = button.clone();
    let anchor = RawMenuAnchor::new(controller.clone(), |_, info| {
        let bounds = info.anchor_rect;
        Stack::new([Widget::from(
            Positioned::new(Widget::box_(Size::new(70., 30.), Color::WHITE))
                .left(bounds.origin.x)
                .top(bounds.origin.y + bounds.size.height),
        )])
        .into()
    })
    .builder(move |context, argument, _| {
        assert!(MenuController::maybe_of(context).as_ref() == Some(argument));
        let is_open = argument.is_open();
        let color = if is_open { CLOSE_BUTTON } else { OPEN_BUTTON };
        let controller = argument.clone();
        Widget::from(
            GestureDetector::new(Widget::box_(Size::new(100., 30.), color)).on_tap(move || {
                if controller.is_open() {
                    controller.close();
                } else {
                    controller.open(None);
                }
            }),
        )
        .with_key(build_key.clone())
    })
    .on_open({
        let count = opened.clone();
        move || count.set(count.get() + 1)
    })
    .on_close({
        let count = closed.clone();
        move || count.set(count.get() + 1)
    });
    let mut runtime = Runtime::new(Center::new(anchor).into()).unwrap();
    let paint = |runtime: &mut Runtime| {
        let list = runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap()
            .0;
        list.commands()
            .iter()
            .filter_map(|cmd| match cmd {
                PaintCommand::Rect { color, .. }
                    if *color == OPEN_BUTTON || *color == CLOSE_BUTTON =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let tap = |runtime: &mut Runtime| {
        let element = runtime.tree().element_with_key(&button).unwrap();
        let bounds = runtime.tree().element_bounds(element).unwrap();
        let position = bounds.origin + Offset::new(10., 10.);
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
    };

    assert_eq!(paint(&mut runtime), [OPEN_BUTTON]);
    assert_eq!(
        (controller.is_open(), opened.get(), closed.get()),
        (false, 0, 0)
    );
    tap(&mut runtime);
    assert_eq!(paint(&mut runtime), [CLOSE_BUTTON]);
    assert_eq!(
        (controller.is_open(), opened.get(), closed.get()),
        (true, 1, 0)
    );
    tap(&mut runtime);
    assert_eq!(paint(&mut runtime), [OPEN_BUTTON]);
    assert_eq!(
        (controller.is_open(), opened.get(), closed.get()),
        (false, 1, 1)
    );
}
