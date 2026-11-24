//! Retained MouseRegion counterparts from Flutter 3.50.0 mouse_region_test.
//! Normalize the initial mouse location using a host pointer sample; do not
//! equate Flutter's positioned PointerAddedEvent with a native window enter.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_widgets::{Align, Alignment, MouseRegion, Widget, internal::Key};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Enter,
    Hover,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Sample {
    kind: Kind,
    version: u8,
    global: Offset,
    local: Offset,
}

#[derive(Default)]
struct Scene {
    samples: Rc<RefCell<Vec<Sample>>>,
}

impl Scene {
    fn widget(&self, alignment: Alignment, version: Option<u8>) -> Widget {
        let child = Widget::box_(Size::new(100., 100.), Color::WHITE);
        let child = if let Some(version) = version {
            let enter = Rc::clone(&self.samples);
            let hover = Rc::clone(&self.samples);
            let exit = Rc::clone(&self.samples);
            Widget::from(
                MouseRegion::new(child)
                    .on_enter(move |event| {
                        enter.borrow_mut().push(Sample {
                            kind: Kind::Enter,
                            version,
                            global: event.position,
                            local: event.local_position,
                        })
                    })
                    .on_hover(move |event| {
                        hover.borrow_mut().push(Sample {
                            kind: Kind::Hover,
                            version,
                            global: event.position,
                            local: event.local_position,
                        })
                    })
                    .on_exit(move |event| {
                        exit.borrow_mut().push(Sample {
                            kind: Kind::Exit,
                            version,
                            global: event.position,
                            local: event.local_position,
                        })
                    }),
            )
            .with_key(Key::from("mouse region"))
        } else {
            child
        };
        Align::new(alignment, child).into()
    }

    fn take(&self) -> Vec<Sample> {
        self.samples.take()
    }
}

struct Host {
    runtime: Runtime,
    now: Instant,
}

impl Host {
    fn mount(widget: Widget) -> Self {
        let mut host = Self {
            runtime: Runtime::new(widget).expect("mount mouse scene"),
            now: Instant::now(),
        };
        host.frame();
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.now)
            .expect("lay out mouse region");
        self.now += Duration::from_millis(16);
    }

    fn update(&mut self, widget: Widget) {
        let root = self.runtime.tree().root().expect("stable alignment root");
        self.runtime
            .tree_mut()
            .update(root, widget)
            .expect("update mouse scene");
        self.frame();
    }

    fn move_mouse(&mut self, position: Offset) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Mouse,
                buttons: 0,
                button: None,
                sample: PointerSampleMetadata::default(),
                phase: PointerPhase::Move,
                position,
            },
            self.now,
        );
        self.frame();
    }
}

#[test]
fn unmounting_hovered_mouse_region_does_not_send_exit() {
    let scene = Scene::default();
    let mut host = Host::mount(scene.widget(Alignment::CENTER, Some(1)));
    host.move_mouse(Offset::new(400., 300.));
    assert_eq!(
        scene
            .take()
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [Kind::Enter, Kind::Hover]
    );

    host.update(scene.widget(Alignment::CENTER, None));
    host.frame();
    assert!(
        host.runtime
            .tree()
            .element_with_key(&Key::from("mouse region"))
            .is_none()
    );
    assert!(scene.take().is_empty());
}

#[test]
fn stationary_mouse_enters_region_moved_under_it_with_local_position() {
    let scene = Scene::default();
    let global = Offset::new(401., 301.);
    let mut host = Host::mount(scene.widget(Alignment::TOP_LEFT, Some(1)));
    host.move_mouse(global);
    assert!(scene.take().is_empty());
    let key = Key::from("mouse region");
    let before = host
        .runtime
        .tree()
        .element_with_key(&key)
        .expect("region mounted");
    host.update(scene.widget(Alignment::CENTER, Some(1)));
    host.frame();
    assert_eq!(host.runtime.tree().element_with_key(&key), Some(before));
    assert_eq!(
        scene.take(),
        [Sample {
            kind: Kind::Enter,
            version: 1,
            global,
            local: Offset::new(51., 51.)
        }]
    );
}

#[test]
fn stationary_mouse_exits_region_moved_away_using_previous_local_position() {
    let scene = Scene::default();
    let global = Offset::new(400., 300.);
    let mut host = Host::mount(scene.widget(Alignment::CENTER, Some(1)));
    host.move_mouse(global);
    assert_eq!(
        scene
            .take()
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [Kind::Enter, Kind::Hover]
    );
    let key = Key::from("mouse region");
    let before = host
        .runtime
        .tree()
        .element_with_key(&key)
        .expect("region mounted");
    host.update(scene.widget(Alignment::TOP_LEFT, Some(1)));
    host.frame();
    assert_eq!(host.runtime.tree().element_with_key(&key), Some(before));
    assert_eq!(
        scene.take(),
        [Sample {
            kind: Kind::Exit,
            version: 1,
            global,
            local: Offset::new(50., 50.)
        }]
    );
}

#[test]
fn retained_mouse_region_uses_replaced_enter_hover_and_exit_callbacks() {
    let scene = Scene::default();
    let mut host = Host::mount(scene.widget(Alignment::TOP_LEFT, Some(1)));
    let key = Key::from("mouse region");
    let before = host
        .runtime
        .tree()
        .element_with_key(&key)
        .expect("region mounted");
    for version in [1, 2] {
        if version == 2 {
            host.update(scene.widget(Alignment::TOP_LEFT, Some(version)));
            assert_eq!(host.runtime.tree().element_with_key(&key), Some(before));
            assert!(scene.take().is_empty());
        }
        host.move_mouse(Offset::new(150., 150.));
        assert!(scene.take().is_empty());
        host.move_mouse(Offset::new(50., 50.));
        host.move_mouse(Offset::new(150., 150.));
        let calls = scene
            .take()
            .into_iter()
            .map(|sample| (sample.kind, sample.version))
            .collect::<Vec<_>>();
        assert_eq!(
            calls,
            [
                (Kind::Enter, version),
                (Kind::Hover, version),
                (Kind::Exit, version)
            ]
        );
    }
}
