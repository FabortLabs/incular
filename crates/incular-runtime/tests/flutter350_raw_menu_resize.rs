use incular_config::{Constraints, RuntimeEnvironment};
use incular_core::{Color, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::{Runtime, Signal};
use incular_widgets::{MediaQuery, MenuController, RawMenuAnchor, Widget};

const PANEL: Color = Color::rgba(17, 36, 74, 255);
const LIMITS: Size = Size {
    width: 450.,
    height: 400.,
};

fn menu(controller: &MenuController, requests: &Signal<usize>, media_size: Size) -> Widget {
    let anchor = RawMenuAnchor::new(controller.clone(), |_, _| {
        Widget::box_(Size::new(70., 50.), PANEL)
    })
    .child(Widget::box_(Size::new(30., 20.), Color::WHITE))
    .on_close_requested({
        let requests = requests.clone();
        move |hide| {
            // Users may update their own state in this callback; it cannot
            // run from inside the read-only widget builder.
            requests.set(requests.get() + 1);
            hide();
        }
    });
    MediaQuery::new(
        RuntimeEnvironment {
            viewport: media_size,
            ..RuntimeEnvironment::default()
        },
        anchor,
    )
    .into()
}

fn frame(runtime: &mut Runtime) -> DisplayList {
    runtime.run_frame(Constraints::tight(LIMITS)).unwrap().0
}

fn panel_visible(list: &DisplayList) -> bool {
    list.commands()
        .iter()
        .any(|command| matches!(command, PaintCommand::Rect { color, .. } if *color == PANEL))
}

#[test]
fn a_media_view_resize_closes_an_open_menu_after_the_build_but_never_a_closed_one() {
    let controller = MenuController::new();
    let requests = Signal::new(0usize);
    let mut runtime = Runtime::new(menu(&controller, &requests, LIMITS)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();

    let update = |runtime: &mut Runtime, side| {
        runtime
            .schedule_update(root, menu(&controller, &requests, Size::new(side, side)))
            .unwrap();
        frame(runtime);
    };
    update(&mut runtime, 250.);
    assert_eq!(requests.get(), 0);
    controller.open(None);
    assert!(panel_visible(&frame(&mut runtime)));
    assert_eq!(requests.get(), 0);

    // The actual surface bounds are unchanged: only MediaQuery.size changes.
    update(&mut runtime, 200.);
    assert_eq!(requests.get(), 1);
    assert!(!controller.is_open());
    assert!(!panel_visible(&frame(&mut runtime)));

    update(&mut runtime, 300.);
    assert_eq!(requests.get(), 1);
}

#[test]
fn resizing_an_open_menu_callbacks_without_mutating_during_build() {
    for grouped in [false, true] {
        check_resize_callbacks(grouped);
    }
}

fn check_resize_callbacks(grouped: bool) {
    use incular_widgets::RawMenuAnchorGroup;
    let group = MenuController::new();
    let menu = MenuController::new();
    let counts = Signal::new((0usize, 0usize));
    let build = |size| {
        let anchor = RawMenuAnchor::new(menu.clone(), |_, _| {
            Widget::box_(Size::new(70., 50.), PANEL)
        })
        .child(Widget::box_(Size::new(30., 20.), Color::WHITE))
        .on_open({
            let counts = counts.clone();
            move || {
                let (open, close) = counts.get();
                counts.set((open + 1, close));
            }
        })
        .on_close({
            let counts = counts.clone();
            move || {
                let (open, close) = counts.get();
                counts.set((open, close + 1));
            }
        });
        let content = if grouped {
            Widget::from(RawMenuAnchorGroup::new(group.clone(), anchor))
        } else {
            Widget::from(anchor)
        };
        Widget::from(MediaQuery::new(
            RuntimeEnvironment {
                viewport: size,
                ..RuntimeEnvironment::default()
            },
            content,
        ))
    };
    let mut runtime = Runtime::new(build(LIMITS)).unwrap();
    assert!(!panel_visible(&frame(&mut runtime)));
    menu.open(None);
    assert!(panel_visible(&frame(&mut runtime)));
    assert_eq!(group.is_open(), grouped);
    assert!(menu.is_open());
    assert_eq!(counts.get(), (1, 0));

    let small = Size::new(200., 200.);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(small)).unwrap();
    runtime.run_frame(Constraints::tight(small)).unwrap();
    assert_eq!(counts.get(), (1, 1));
    assert!(!group.is_open() && !menu.is_open());
    let (list, _) = runtime.run_frame(Constraints::tight(small)).unwrap();
    assert!(!panel_visible(&list));
    assert_eq!(counts.get(), (1, 1));
}
