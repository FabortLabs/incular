//! Pinned Flutter 3.50.0 widgets/mouse_region_test.dart comparisons.
//! Pointer samples and effective cursor observations are headless.

use std::{cell::Cell, rc::Rc};

use incular_config::{Alignment, Constraints};
use incular_core::{InputEvent, Offset, PointerDeviceKind, PointerPhase, Size};
use incular_platform::PlatformEvent;
use incular_rendering::DisplayList;
use incular_runtime::Runtime;
use incular_widgets::{
    Align, CustomPaint, CustomPainter, MouseCursor, MouseRegion, SizedBox, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn move_mouse(runtime: &mut Runtime, x: f32, y: f32) {
    let _ = runtime.handle_platform_event(PlatformEvent::Input(InputEvent::PointerWithMetadata {
        pointer: 0,
        device: 1,
        kind: PointerDeviceKind::Mouse,
        buttons: 0,
        button: None,
        sample: Default::default(),
        phase: PointerPhase::Move,
        position: Offset::new(x, y),
    }));
}

fn top_left(child: impl Into<Widget>) -> Widget {
    Align::new(Alignment::TOP_LEFT, child).into()
}

fn update(runtime: &mut Runtime, widget: Widget) {
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, widget).unwrap();
    frame(runtime);
}

// Upstream :667. A positioned sample establishes the simulated mouse in
// Incular; the cursor is the runtime's effective native-boundary result.
#[test]
fn cursor_uses_basic_outside_and_text_inside_the_region() {
    let root = top_left(
        MouseRegion::new(SizedBox::new().width(10.).height(10.)).cursor(MouseCursor::Text),
    );
    let mut runtime = Runtime::new(root).unwrap();
    frame(&mut runtime);
    move_mouse(&mut runtime, 100., 100.);
    frame(&mut runtime);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Basic);
    move_mouse(&mut runtime, 5., 5.);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Text);
    move_mouse(&mut runtime, 100., 100.);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Basic);
}

#[derive(Default)]
struct Logs {
    enters: Cell<usize>,
    paints: Cell<usize>,
}

struct CountingPainter(Rc<Logs>);
impl CustomPainter for CountingPainter {
    fn paint(&self, _: Size) -> DisplayList {
        self.0.paints.set(self.0.paints.get() + 1);
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false // Flutter's delegates in both registrations use the same null key.
    }
}

fn paint_region(
    logs: &Rc<Logs>,
    child_cursor: Option<MouseCursor>,
    nested: bool,
    observe_enter: bool,
) -> Widget {
    let mut region = MouseRegion::new(CustomPaint::from_painter(
        Size::new(10., 10.),
        CountingPainter(logs.clone()),
    ));
    if let Some(cursor) = child_cursor {
        region = region.cursor(cursor);
    }
    if observe_enter {
        let logs = logs.clone();
        region = region.on_enter(move |_| logs.enters.set(logs.enters.get() + 1));
    }
    if nested {
        top_left(MouseRegion::new(region).cursor(MouseCursor::NotAllowed))
    } else {
        top_left(region)
    }
}

fn sample(runtime: &Runtime, logs: &Logs) -> (MouseCursor, usize, usize) {
    (
        runtime.effective_mouse_cursor(),
        logs.enters.replace(0),
        logs.paints.replace(0),
    )
}

// Upstream :1665.
#[test]
fn changing_cursor_under_stationary_mouse_updates_it_without_reentering() {
    let logs = Rc::new(Logs::default());
    let mut runtime = Runtime::new(paint_region(
        &logs,
        Some(MouseCursor::NotAllowed),
        false,
        true,
    ))
    .unwrap();
    move_mouse(&mut runtime, 100., 100.);
    frame(&mut runtime);
    move_mouse(&mut runtime, 5., 5.);
    let before = sample(&runtime, &logs);

    update(
        &mut runtime,
        paint_region(&logs, Some(MouseCursor::Text), false, true),
    );
    let after = sample(&runtime, &logs);
    assert_eq!(
        [before, after],
        [(MouseCursor::NotAllowed, 1, 1), (MouseCursor::Text, 0, 1)]
    );
}

// Upstream :1726. Omission uses the default defer cursor.
#[test]
fn default_cursor_defers_to_parent_under_stationary_mouse() {
    let logs = Rc::new(Logs::default());
    let mut runtime =
        Runtime::new(paint_region(&logs, Some(MouseCursor::Text), true, true)).unwrap();
    move_mouse(&mut runtime, 100., 100.);
    frame(&mut runtime);
    move_mouse(&mut runtime, 5., 5.);
    let explicit = sample(&runtime, &logs);

    update(&mut runtime, paint_region(&logs, None, true, true));
    let deferred = sample(&runtime, &logs);
    update(
        &mut runtime,
        paint_region(&logs, Some(MouseCursor::Text), true, false),
    );
    let restored = sample(&runtime, &logs);
    assert_eq!(
        [explicit, deferred, restored],
        [
            (MouseCursor::Text, 1, 1),
            (MouseCursor::NotAllowed, 0, 1),
            (MouseCursor::Text, 0, 1),
        ]
    );
}

// Upstream :1971.
#[test]
fn mouse_move_with_no_callback_side_effects_does_not_request_another_frame() {
    let root = Align::new(
        Alignment::CENTER,
        MouseRegion::new(SizedBox::new().width(100.).height(100.))
            .on_enter(|_| {})
            .on_hover(|_| {})
            .on_exit(|_| {}),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    move_mouse(&mut runtime, 400., 300.);
    frame(&mut runtime);
    assert!(!runtime.frame_requested(), "initial pointer is settled");
    move_mouse(&mut runtime, 410., 310.);
    assert!(!runtime.frame_requested(), "pointer move is visually inert");
}
