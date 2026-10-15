//! `DocumentEditor` lays out only the lines an edit changes; `TextArea`
//! keeps Flutter's whole-text layout.

use incular_config::Constraints;
use incular_controls::prelude::*;
use incular_core::Size;
use incular_widgets::{
    Directionality, TextDirection, Widget,
    internal::{TextEditingController, WidgetTree},
};

fn document() -> String {
    (0..2000)
        .map(|index| format!("let line_{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Paragraphs laid out again when one character is typed at the top.
fn paragraphs_laid_out_for_a_top_edit(editor: impl FnOnce(TextEditingController) -> Widget) -> u64 {
    let controller = TextEditingController::with_text(document());
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, editor(controller.clone())).into())
        .expect("mount editor");
    let constraints = Constraints::tight(Size::new(640., 480.));
    tree.layout(constraints).expect("layout");
    let before = tree.text_diagnostics();
    controller.set_text(format!("x{}", document()));
    tree.layout(constraints).expect("layout after edit");
    tree.text_diagnostics().paragraphs_considered - before.paragraphs_considered
}

#[test]
fn a_top_edit_lays_out_only_the_edited_line() {
    let laid_out = paragraphs_laid_out_for_a_top_edit(|controller| {
        DocumentEditor::new(controller)
            .size(Size::new(640., 480.))
            .into()
    });
    assert!(
        laid_out <= 2,
        "DocumentEditor laid out {laid_out} paragraphs"
    );
}

#[test]
fn text_area_keeps_the_whole_text_layout() {
    let laid_out = paragraphs_laid_out_for_a_top_edit(|controller| {
        TextArea::new(controller).size(Size::new(640., 480.)).into()
    });
    assert!(laid_out >= 2000, "TextArea laid out {laid_out} paragraphs");
}

#[test]
fn document_editor_paints_its_text() {
    let controller = TextEditingController::with_text("first\nsecond");
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            DocumentEditor::new(controller).size(Size::new(320., 200.)),
        )
        .into(),
    )
    .expect("mount editor");
    tree.layout(Constraints::loose(Size::new(400., 300.)))
        .expect("layout");
    let runs = tree
        .paint()
        .commands()
        .iter()
        .filter(|command| matches!(command, incular_rendering::PaintCommand::GlyphRun { .. }))
        .count();
    assert!(runs >= 2, "both lines paint");
}
