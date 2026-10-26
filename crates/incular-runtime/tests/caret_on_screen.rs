//! Flutter's `EditableTextState._showCaretOnScreen`: a focused field scrolls
//! its ancestor viewport so the caret, inflated by `scrollPadding`, is
//! visible (`RenderViewportBase.showInViewport`).

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, EditableText, SingleChildScrollView, Widget,
    internal::{TextEditingController, TextSelection},
};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(300., 200.))
}

/// A field 500 px down a 200 px vertical scroll view.
fn mount(padding: Option<EdgeInsets>) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text("hello");
    let mut field = EditableText::new(controller.clone())
        .text_direction(incular_widgets::TextDirection::Ltr)
        .size(Size::new(300., 32.));
    if let Some(padding) = padding {
        field = field.scroll_padding(padding);
    }
    let content = Column::new(vec![
        Widget::box_(Size::new(300., 500.), incular_core::Color::WHITE),
        Widget::from(field),
        Widget::box_(Size::new(300., 500.), incular_core::Color::WHITE),
    ]);
    let mut runtime = Runtime::new(SingleChildScrollView::new(content).into()).unwrap();
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

fn focus_field(runtime: &mut Runtime) {
    let tab = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    let _ = runtime.handle_input(InputEvent::Key(tab));
    for _ in 0..3 {
        runtime.run_frame(bounds()).unwrap();
    }
}

fn caret_bottom(runtime: &Runtime) -> f32 {
    let field = runtime.focused_element().expect("focused field");
    let caret = runtime
        .tree()
        .text_field_input_snapshot(field)
        .expect("snapshot")
        .caret_rect;
    caret.origin.y + caret.size.height
}

#[test]
fn focusing_a_field_below_the_viewport_reveals_its_caret_with_padding() {
    let (mut runtime, _) = mount(None);
    focus_field(&mut runtime);
    // The caret was below the viewport, so it scrolls to the trailing edge:
    // its bottom plus the default 20 px scroll padding meets the bottom.
    assert!((caret_bottom(&runtime) - 180.).abs() < 0.01);
}

#[test]
fn scroll_padding_sets_the_margin() {
    let (mut runtime, _) = mount(Some(EdgeInsets::all(50.)));
    focus_field(&mut runtime);
    assert!((caret_bottom(&runtime) - 150.).abs() < 0.01);
}

#[test]
fn a_visible_caret_does_not_scroll() {
    let (mut runtime, controller) = mount(None);
    focus_field(&mut runtime);
    let before = caret_bottom(&runtime);
    // Moving the caret within the visible line keeps the scroll offset.
    controller.set_selection(TextSelection::collapsed(2));
    for _ in 0..2 {
        runtime.run_frame(bounds()).unwrap();
    }
    assert!((caret_bottom(&runtime) - before).abs() < 0.01);
}
