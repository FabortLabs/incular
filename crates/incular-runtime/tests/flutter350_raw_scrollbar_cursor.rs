use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind as Kind,
    PointerPhase as Phase, PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_scroll::ScrollController;
use incular_widgets::{
    Directionality, MouseCursor as Cursor, MouseRegion, RawScrollbar, RawScrollbarStyle,
    ScrollBehavior, ScrollConfiguration, ScrollbarCursorState, SingleChildScrollView, Widget,
};

const THUMB: Offset = Offset::new(796., 5.);
const CONTENT: Offset = Offset::new(100., 100.);

type Resolver = Rc<dyn Fn(ScrollbarCursorState) -> Option<Cursor>>;

fn fallback_map() -> Resolver {
    Rc::new(|state| {
        if state.dragged {
            Some(Cursor::Grabbing)
        } else if state.hovered {
            Some(Cursor::Grab)
        } else {
            None
        }
    })
}

fn viewport(
    controller: &ScrollController,
    content: f32,
    interactive: bool,
    resolve: Option<Resolver>,
) -> Widget {
    let view = SingleChildScrollView::new(Widget::box_(Size::new(4000., content), Color::WHITE))
        .controller(controller.clone());
    let mut bar = RawScrollbar::new(controller.clone());
    bar.set_style(RawScrollbarStyle {
        thumb_visibility: Some(true),
        interactive,
        ..RawScrollbarStyle::default()
    });
    if let Some(resolve) = resolve {
        bar = bar.mouse_cursor_with_states(move |state| resolve(state));
    }
    Directionality::new(
        TextDirection::Ltr,
        ScrollConfiguration::new(MouseRegion::new(bar.with_child(view)).cursor(Cursor::Text))
            .behavior(ScrollBehavior::new().scrollbars(false)),
    )
    .into()
}

struct Scene {
    runtime: Runtime,
    controller: ScrollController,
    at: Instant,
}

impl Scene {
    fn new(resolve: Option<Resolver>) -> Self {
        let controller = ScrollController::new();
        let root = viewport(&controller, 4000., true, resolve);
        let mut scene = Self {
            runtime: Runtime::new(root).unwrap(),
            controller,
            at: Instant::now(),
        };
        scene.frame();
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.at += Duration::from_millis(50);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.at)
            .unwrap();
    }

    fn update(&mut self, content: f32, interactive: bool, resolver: Option<Resolver>) {
        let id = self.runtime.tree().root().unwrap();
        self.runtime
            .schedule_update(
                id,
                viewport(&self.controller, content, interactive, resolver),
            )
            .unwrap();
        self.frame();
    }

    fn send(&mut self, kind: Kind, phase: Phase, position: Offset, held: bool) {
        let id = if kind == Kind::Mouse { 1 } else { 2 };
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                device: id,
                pointer: id,
                kind,
                position,
                phase,
                buttons: if held { PRIMARY_POINTER_BUTTON } else { 0 },
                button: matches!(phase, Phase::Down | Phase::Up).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
            },
            self.at + Duration::from_millis(5),
        );
        self.frame();
    }

    fn mouse(&mut self, phase: Phase, position: Offset, held: bool) {
        self.send(Kind::Mouse, phase, position, held);
    }

    fn cursor(&self) -> Cursor {
        self.runtime.mouse_cursor_for_device(1, 1).unwrap()
    }
}

#[test]
fn scrollbar_claims_only_the_track_and_reports_hover_exit_to_its_resolver() {
    let states = Rc::new(RefCell::new(Vec::new()));
    let recorded = states.clone();
    let mut scene = Scene::new(Some(Rc::new(move |state| {
        recorded.borrow_mut().push(state);
        Some(Cursor::Basic)
    })));
    scene.mouse(Phase::Move, CONTENT, false);
    assert_eq!(scene.cursor(), Cursor::Text);
    scene.mouse(Phase::Move, THUMB, false);
    assert_eq!(scene.cursor(), Cursor::Basic);
    assert_eq!(
        states.borrow().last(),
        Some(&ScrollbarCursorState {
            hovered: true,
            dragged: false,
            disabled: false
        })
    );
    scene.mouse(Phase::Move, CONTENT, false);
    assert_eq!(scene.cursor(), Cursor::Text);
    assert!(!states.borrow().last().unwrap().hovered);
}

#[test]
fn missing_cursor_defers_and_changed_configuration_updates_a_stationary_mouse() {
    let mut scene = Scene::new(None);
    scene.mouse(Phase::Move, THUMB, false);
    assert_eq!(scene.cursor(), Cursor::Text);
    scene.update(4000., true, Some(Rc::new(|_| Some(Cursor::Basic))));
    assert_eq!(scene.cursor(), Cursor::Basic);
    scene.update(4000., true, Some(Rc::new(|_| Some(Cursor::Grab))));
    assert_eq!(scene.cursor(), Cursor::Grab);
}

#[test]
fn resolving_dragged_before_hovered_keeps_cursor_until_release() {
    let mut scene = Scene::new(Some(fallback_map()));
    scene.mouse(Phase::Move, CONTENT, false);
    assert_eq!(scene.cursor(), Cursor::Text);
    scene.mouse(Phase::Move, THUMB, false);
    assert_eq!(scene.cursor(), Cursor::Grab);
    scene.mouse(Phase::Down, THUMB, true);
    scene.mouse(Phase::Move, THUMB + Offset::new(0., 10.), true);
    assert_eq!(scene.cursor(), Cursor::Grabbing);
    scene.mouse(Phase::Up, THUMB + Offset::new(0., 10.), false);
    assert_eq!(scene.cursor(), Cursor::Grab);

    let top = THUMB + Offset::new(0., 10.);
    scene.mouse(Phase::Down, top, true);
    scene.mouse(Phase::Move, CONTENT, true);
    assert_eq!(scene.cursor(), Cursor::Grabbing);
    scene.mouse(Phase::Up, CONTENT, false);
    assert_eq!(scene.cursor(), Cursor::Text);
}

#[test]
fn touch_drag_keeps_the_mouse_override_off_track_and_cancel_restores_hover() {
    let mut scene = Scene::new(Some(fallback_map()));
    scene.mouse(Phase::Move, THUMB, false);
    assert_eq!(scene.cursor(), Cursor::Grab);
    let touch_start = THUMB + Offset::new(0., 1.);
    let touch_move = touch_start + Offset::new(0., 5.);
    scene.send(Kind::Touch, Phase::Down, touch_start, true);
    scene.send(Kind::Touch, Phase::Move, touch_move, true);
    assert_eq!(scene.cursor(), Cursor::Grabbing);
    scene.mouse(Phase::Move, CONTENT, false);
    assert_eq!(scene.cursor(), Cursor::Grabbing);
    scene.mouse(Phase::Move, THUMB, false);
    scene.send(Kind::Touch, Phase::Cancel, touch_move, false);
    assert_eq!(scene.cursor(), Cursor::Grab);
}

#[test]
fn disabling_hover_or_drag_releases_the_override_without_a_new_pointer_packet() {
    let mut scene = Scene::new(Some(fallback_map()));
    scene.mouse(Phase::Move, THUMB, false);
    assert_eq!(scene.cursor(), Cursor::Grab);
    scene.update(4000., false, Some(fallback_map()));
    assert_eq!(scene.cursor(), Cursor::Text);
    scene.update(4000., true, Some(fallback_map()));
    scene.mouse(Phase::Move, THUMB, false);
    scene.mouse(Phase::Down, THUMB, true);
    assert_eq!(scene.cursor(), Cursor::Grabbing);
    scene.update(4000., false, Some(fallback_map()));
    assert_eq!(scene.cursor(), Cursor::Text);
    scene.mouse(Phase::Up, THUMB, false);
    assert_eq!(scene.cursor(), Cursor::Text);
}

#[test]
fn removing_scrollable_range_clears_dragged_while_preserving_stationary_hover() {
    let mut scene = Scene::new(Some(fallback_map()));
    scene.mouse(Phase::Move, THUMB, false);
    scene.mouse(Phase::Down, THUMB, true);
    let moved = THUMB + Offset::new(0., 10.);
    scene.mouse(Phase::Move, moved, true);
    assert_eq!(scene.cursor(), Cursor::Grabbing);
    scene.update(10., true, Some(fallback_map()));
    assert_eq!(scene.cursor(), Cursor::Grab);
    scene.mouse(Phase::Up, moved, false);
    scene.mouse(Phase::Move, CONTENT, false);
    assert_eq!(scene.cursor(), Cursor::Text);
}
