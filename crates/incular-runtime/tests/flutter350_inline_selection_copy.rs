//! Flutter 3.50.0 SelectableRegion copy across a WidgetSpan on one line.
use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
};
use incular_platform::{MemoryClipboard, TransferFormat, TransferReadRequest};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{RichText, TextSpan, TextStyle, WidgetSpan, WidgetSpanAlignment};
use incular_widgets::{
    Align, Directionality, RichParagraph, SelectionAreaController, Text, Widget,
};

#[test]
fn copy_select_all_interleaves_inline_widget_text_in_the_paragraph() {
    let marker = 43;
    let style = TextStyle::new().font_size(10.);
    let span = TextSpan::empty()
        .child(TextSpan::new("Hello my name is ").style(style.clone()))
        .child(WidgetSpan::new(marker).alignment(WidgetSpanAlignment::Middle))
        .child(TextSpan::new(".").style(style.clone()));
    let paragraph = Widget::from(
        RichParagraph::new(RichText::new(span)).child(
            marker,
            Text::new("Dash")
                .text_direction(TextDirection::Ltr)
                .style(style),
        ),
    )
    .with_key(77_u64);
    let controller = SelectionAreaController::new();
    let content = Directionality::new(TextDirection::Ltr, Align::new(Alignment::CENTER, paragraph));
    let mut runtime = Runtime::new(Widget::selection_area(controller.clone(), content.into()))
        .expect("retained region");
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime.set_clipboard(Box::<MemoryClipboard>::default());
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .expect("lay out the inline Text");

    let target = runtime.tree().element_with_key(&77_u64.into()).unwrap();
    let box_ = runtime.tree().element_bounds(target).unwrap();
    let position = box_.origin + Offset::new(0.5, box_.size.height * 0.5);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    }
    assert_eq!(runtime.focused_element(), Some(target));
    for key in [Code::KeyA, Code::KeyC] {
        let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), key);
        event.modifiers = Modifiers::CONTROL;
        let _ = runtime.handle_input(InputEvent::Key(event));
    }

    let clipboard = runtime
        .read_clipboard(TransferReadRequest::format(TransferFormat::PlainText))
        .expect("copy wrote plain text");
    const EXPECTED: &str = "Hello my name is Dash.";
    assert_eq!(
        clipboard.plain_text_value().unwrap().as_deref(),
        Some(EXPECTED)
    );
    assert_eq!(controller.selected_text(), EXPECTED);
    assert_eq!(
        controller.selection().range,
        Some(incular_widgets::SelectedContentRange::new(0, EXPECTED.len())),
    );
}
