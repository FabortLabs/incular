//! Real retained RawScrollbar input using the pinned Flutter geometry.
use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size,
};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_scroll::ScrollController;
use incular_widgets::{
    Directionality, RawScrollbar, RawScrollbarStyle, ScrollBehavior, ScrollConfiguration,
    SingleChildScrollView, Widget,
};

const SIZE: Size = Size {
    width: 800.,
    height: 600.,
};
const INK: Color = Color::rgba(188, 188, 188, 102);

struct Scene {
    runtime: Runtime,
    controller: ScrollController,
    at: Instant,
}

impl Scene {
    fn new(content: f32, always_visible: bool) -> Self {
        let controller = ScrollController::new();
        let view =
            SingleChildScrollView::new(Widget::box_(Size::new(content, content), Color::WHITE))
                .controller(controller.clone());
        let mut scrollbar = RawScrollbar::new(controller.clone());
        scrollbar.set_style(RawScrollbarStyle {
            thumb_visibility: Some(always_visible),
            ..RawScrollbarStyle::default()
        });
        // An explicit overlay must work even when there is no automatic
        // desktop scrollbar underneath it.
        let widget = ScrollConfiguration::new(scrollbar.with_child(view))
            .behavior(ScrollBehavior::new().scrollbars(false));
        let runtime = Runtime::new(Directionality::new(TextDirection::Ltr, widget).into()).unwrap();
        let mut scene = Self {
            runtime,
            controller,
            at: Instant::now(),
        };
        scene.frame(0);
        scene
    }

    fn frame(&mut self, ms: u64) -> DisplayList {
        self.runtime
            .run_frame_at(
                Constraints::tight(SIZE),
                self.at + Duration::from_millis(ms),
            )
            .unwrap()
            .0
    }

    fn pointer(&mut self, ms: u64, phase: PointerPhase, kind: PointerDeviceKind, x: f32, y: f32) {
        let contact = kind == PointerDeviceKind::Touch;
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: if contact { 2 } else { 1 },
                phase,
                position: Offset::new(x, y),
                kind,
                buttons: u32::from(
                    contact && matches!(phase, PointerPhase::Down | PointerPhase::Move),
                ),
                button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                    .then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
            },
            self.at + Duration::from_millis(ms),
        );
    }

    fn touch(&mut self, ms: u64, phase: PointerPhase, y: f32) {
        self.pointer(ms, phase, PointerDeviceKind::Touch, 797., y);
    }
}

fn thumb(list: &DisplayList) -> Option<Rect> {
    list.commands()
        .iter()
        .rev()
        .find_map(|command| match command {
            PaintCommand::Rect { rect, color } if *color == INK => Some(*rect),
            _ => None,
        })
}

fn rect(top: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(794., top), Size::new(6., height))
}

#[test]
fn the_retained_thumb_captures_touch_and_moves_ten_pixels() {
    let mut scene = Scene::new(4000., true);
    assert_eq!(thumb(&scene.frame(300)), Some(rect(0., 90.)));
    assert_eq!(scene.controller.offset(), 0.);
    scene.touch(310, PointerPhase::Down, 45.);
    scene.touch(340, PointerPhase::Move, 55.);
    scene.touch(370, PointerPhase::Up, 55.);
    assert!(scene.controller.offset() > 20.);
    assert_eq!(thumb(&scene.frame(400)), Some(rect(10., 90.)));
    assert!(scene.controller.current_activity_id().is_none());
}

#[test]
fn clicking_either_side_of_the_thumb_pages_on_release() {
    let mut scene = Scene::new(1000., true);
    assert_eq!(thumb(&scene.frame(300)), Some(rect(0., 360.)));
    scene.touch(310, PointerPhase::Down, 550.);
    assert_eq!(
        scene.controller.offset(),
        0.,
        "down has not committed the tap"
    );
    scene.touch(330, PointerPhase::Up, 550.);
    scene.frame(330);
    scene.frame(380);
    assert!((scene.controller.offset() - 200.).abs() < 0.01);
    assert_eq!(thumb(&scene.frame(450)), Some(rect(240., 360.)));
    assert_eq!(scene.controller.offset(), 400.);
    scene.touch(600, PointerPhase::Down, 50.);
    scene.touch(620, PointerPhase::Up, 50.);
    scene.frame(620);
    assert_eq!(thumb(&scene.frame(750)), Some(rect(0., 360.)));
    assert_eq!(scene.controller.offset(), 0.);
}

#[test]
fn a_mouse_can_reveal_the_thumb_before_any_scroll_updates() {
    let mut scene = Scene::new(4000., false);
    assert_eq!(thumb(&scene.frame(300)), None);
    scene.pointer(310, PointerPhase::Move, PointerDeviceKind::Mouse, 794., 5.);
    scene.frame(310);
    assert_eq!(thumb(&scene.frame(620)), Some(rect(0., 90.)));
    assert_eq!(scene.controller.offset(), 0.);
}

#[test]
fn a_track_page_moves_by_eighty_percent_of_the_viewport_and_rejects_canceled_taps() {
    let mut scene = Scene::new(4000., true);
    scene.frame(300);
    scene.touch(310, PointerPhase::Down, 500.);
    scene.touch(320, PointerPhase::Move, 540.);
    scene.touch(330, PointerPhase::Up, 540.);
    scene.frame(450);
    assert_eq!(scene.controller.offset(), 0.);

    scene.touch(500, PointerPhase::Down, 500.);
    scene.touch(510, PointerPhase::Cancel, 500.);
    scene.frame(650);
    assert_eq!(scene.controller.offset(), 0.);

    scene.touch(660, PointerPhase::Down, 500.);
    scene.touch(670, PointerPhase::Up, 500.);
    scene.frame(670);
    scene.frame(790);
    assert!((scene.controller.offset() - 480.).abs() < 0.01);
}

#[test]
fn a_held_track_press_pages_once_before_release() {
    let mut scene = Scene::new(4000., true);
    scene.frame(300);
    scene.touch(310, PointerPhase::Down, 500.);
    scene.frame(409);
    assert_eq!(scene.controller.offset(), 0.);
    scene.frame(410);
    scene.frame(460);
    assert!((230. ..=255.).contains(&scene.controller.offset()));
    scene.frame(520);
    assert_eq!(scene.controller.offset(), 480.);
    scene.touch(530, PointerPhase::Up, 500.);
    scene.frame(530);
    scene.frame(650);
    assert_eq!(scene.controller.offset(), 480.);
}
