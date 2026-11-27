//! The pinned painting/text_span_test.dart mouse annotations, through runtime input.

use std::sync::{Arc, Mutex};

use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerDeviceKind, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_text::{RichText, TextAlign, TextOverflow, TextSpan, TextStyle};
use incular_widgets::{Align, Directionality, MouseCursor, MouseRegion, SizedBox, Widget};

fn ahem() -> Option<TextStyle> {
    let font = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")?;
    const AHEM: &str = "Pinned TextSpan mouse Ahem";
    incular_text::register_application_font_as(AHEM, std::fs::read(font).unwrap());
    Some(TextStyle::new().font_family(AHEM).font_size(10.))
}

fn paragraph(middle: TextSpan) -> Option<Widget> {
    Some(
        RichText::new(
            TextSpan::new("xxxxx")
                .style(ahem()?)
                .child(middle)
                .child(TextSpan::new("xxxxx")),
        )
        .text_align(TextAlign::Center)
        .into(),
    )
}

fn scene(paragraph: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, Align::new(Alignment::CENTER, paragraph)).into()
}

fn mount(paragraph: Widget) -> Runtime {
    let mut runtime = Runtime::new(scene(paragraph)).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

fn move_mouse(runtime: &mut Runtime, x: f32, y: f32) {
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 1,
        device: 1,
        kind: PointerDeviceKind::Mouse,
        buttons: 0,
        button: None,
        sample: Default::default(),
        phase: PointerPhase::Move,
        position: Offset::new(x, y),
    });
}

#[test]
fn only_the_middle_span_changes_the_effective_native_cursor() {
    let Some(paragraph) = paragraph(TextSpan::new("yyyyy").mouse_cursor(MouseCursor::NotAllowed))
    else {
        return;
    };
    let mut runtime = mount(paragraph);
    for (x, y, expected) in [
        (360., 300., MouseCursor::Basic),
        (400., 300., MouseCursor::NotAllowed),
        (440., 300., MouseCursor::Basic),
        (400., 280., MouseCursor::Basic),
    ] {
        move_mouse(&mut runtime, x, y);
        assert_eq!(runtime.effective_mouse_cursor(), expected);
    }
}

#[test]
fn enter_and_exit_fire_only_when_crossing_the_span() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let entering = events.clone();
    let exiting = events.clone();
    let Some(paragraph) = paragraph(
        TextSpan::new("yyyyy")
            .on_enter(move |event| entering.lock().unwrap().push(event.phase))
            .on_exit(move |event| exiting.lock().unwrap().push(event.phase)),
    ) else {
        return;
    };
    let mut runtime = mount(paragraph);
    move_mouse(&mut runtime, 360., 300.);
    assert!(events.lock().unwrap().is_empty());
    move_mouse(&mut runtime, 400., 300.);
    assert_eq!(*events.lock().unwrap(), [PointerPhase::Enter]);
    move_mouse(&mut runtime, 405., 300.);
    assert_eq!(*events.lock().unwrap(), [PointerPhase::Enter]);
    move_mouse(&mut runtime, 440., 300.);
    assert_eq!(
        *events.lock().unwrap(),
        [PointerPhase::Enter, PointerPhase::Exit]
    );
}

#[test]
fn span_cursor_and_transitions_follow_the_enclosing_region_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let span_enter = events.clone();
    let span_exit = events.clone();
    let region_enter = events.clone();
    let region_exit = events.clone();
    let Some(paragraph) = paragraph(
        TextSpan::new("yyyyy")
            .mouse_cursor(MouseCursor::NotAllowed)
            .on_enter(move |_| span_enter.lock().unwrap().push("span enter"))
            .on_exit(move |_| span_exit.lock().unwrap().push("span exit")),
    ) else {
        return;
    };
    let mut runtime = mount(
        MouseRegion::new(paragraph)
            .cursor(MouseCursor::Click)
            .on_enter(move |_| region_enter.lock().unwrap().push("region enter"))
            .on_exit(move |_| region_exit.lock().unwrap().push("region exit"))
            .into(),
    );
    move_mouse(&mut runtime, 360., 300.);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Click);
    move_mouse(&mut runtime, 400., 300.);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::NotAllowed);
    move_mouse(&mut runtime, 400., 325.);
    assert_eq!(
        *events.lock().unwrap(),
        ["region enter", "span enter", "span exit", "region exit"]
    );
}

#[test]
fn ellipsis_does_not_inherit_the_truncated_spans_cursor() {
    let Some(style) = ahem() else {
        return;
    };
    let text = RichText::new(
        TextSpan::new("xxxxx")
            .style(style)
            .child(TextSpan::new("yyyyyyyyyy").mouse_cursor(MouseCursor::NotAllowed)),
    )
    .max_lines(Some(1))
    .overflow(TextOverflow::Ellipsis);
    let mut runtime = mount(SizedBox::new().width(80.).child(text).into());
    move_mouse(&mut runtime, 415., 300.);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::NotAllowed);
    move_mouse(&mut runtime, 435., 300.);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Basic);
}

#[test]
fn unmounting_the_hovered_span_delivers_an_exit() {
    let exits = Arc::new(Mutex::new(Vec::new()));
    let captured = exits.clone();
    let Some(paragraph) = paragraph(
        TextSpan::new("yyyyy").on_exit(move |event| captured.lock().unwrap().push(event)),
    ) else {
        return;
    };
    let mut runtime = mount(paragraph);
    move_mouse(&mut runtime, 400., 300.);
    assert!(exits.lock().unwrap().is_empty());

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, scene(SizedBox::new()))
        .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let events = exits.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].phase, PointerPhase::Exit);
    assert_eq!(events[0].position, Offset::new(400., 300.));
}

#[test]
fn shared_callbacks_do_not_merge_different_spans_but_equal_spans_keep_hover() {
    let Some(style) = ahem() else {
        return;
    };
    for equivalent in [false, true] {
        let events = Arc::new(Mutex::new(Vec::new()));
        let entering = events.clone();
        let exiting = events.clone();
        let left = TextSpan::new("xxxxx")
            .on_enter(move |event| entering.lock().unwrap().push(event.phase))
            .on_exit(move |event| exiting.lock().unwrap().push(event.phase));
        let mut right = left.clone();
        if !equivalent {
            right.text = "yyyyy".into();
        }
        let span = TextSpan::empty()
            .style(style.clone())
            .child(left)
            .child(right);
        let mut runtime = mount(RichText::new(span).into());
        move_mouse(&mut runtime, 380., 300.);
        assert_eq!(*events.lock().unwrap(), [PointerPhase::Enter]);
        move_mouse(&mut runtime, 420., 300.);
        let expected: &[PointerPhase] = if equivalent {
            &[PointerPhase::Enter]
        } else {
            &[PointerPhase::Enter, PointerPhase::Exit, PointerPhase::Enter]
        };
        assert_eq!(*events.lock().unwrap(), expected, "equivalent={equivalent}");
    }
}
