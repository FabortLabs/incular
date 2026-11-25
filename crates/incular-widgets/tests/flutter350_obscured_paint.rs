//! Default obscuring is one bullet per UTF-16 code unit on Flutter.

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_rendering::PaintCommand;
use incular_text::TextAffinity;
use incular_widgets::{
    EditableText, TextEditingController,
    internal::{ElementId, WidgetTree},
};

fn editor(text: &str, obscured: bool) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(TextEditingController::with_text(text))
                .text_direction(TextDirection::Ltr)
                .obscure_text(obscured)
                .into(),
        )
        .unwrap();
    tree.layout(Constraints::tight(Size::new(240., 50.)))
        .unwrap();
    (tree, root)
}

fn glyphs(tree: &mut WidgetTree) -> Vec<u16> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::GlyphRun { run, .. } => Some(run.glyphs.iter().map(|g| g.id)),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn non_ascii_password_paints_the_default_utf16_mask_and_maps_valid_carets() {
    let (mut masked, root) = editor("é👩‍💻", true);
    let (mut bullets, plain) = editor("••••••", false);
    assert_eq!(glyphs(&mut bullets).len(), 6);
    assert_eq!(glyphs(&mut masked), glyphs(&mut bullets));
    masked.update_semantics();
    assert!(
        masked
            .semantics()
            .iter()
            .any(|(_, node)| node.state.obscured && node.value.as_deref() == Some("••••••"))
    );

    for (byte, unit) in [(0, 0), (2, 1), (6, 3), (9, 4), (13, 6)] {
        let caret = masked
            .text_field_offset_rect(root, byte, TextAffinity::Downstream)
            .unwrap();
        let expected = bullets
            .text_field_offset_rect(plain, 3 * unit, TextAffinity::Downstream)
            .unwrap();
        assert_eq!(caret, expected, "UTF-8 byte {byte}, UTF-16 unit {unit}");
        assert_eq!(
            masked
                .text_field_position_at(root, Offset::new(caret.origin.x, caret.origin.y + 2.))
                .unwrap()
                .0,
            byte,
        );
    }
}

#[test]
fn composed_password_masks_the_replacement_and_maps_the_committed_suffix() {
    use incular_text::TextSelection;
    let controller = TextEditingController::with_text("éq💻");
    controller.set_selection(TextSelection::new(2, 3));
    controller.set_preedit("👩‍💻", None);
    let mut masked = WidgetTree::new();
    let root = masked
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .obscure_text(true)
                .into(),
        )
        .unwrap();
    masked
        .layout(Constraints::tight(Size::new(240., 50.)))
        .unwrap();
    let (mut bullets, plain) = editor("••••••••", false);
    assert_eq!(glyphs(&mut masked), glyphs(&mut bullets));
    masked.update_semantics();
    assert_eq!(controller.text(), "éq💻");
    assert!(
        masked
            .semantics()
            .iter()
            .any(|(_, node)| node.state.obscured && node.value.as_deref() == Some("••••••••"))
    );
    assert_eq!(
        masked
            .text_field_offset_rect(root, 7, TextAffinity::Downstream)
            .unwrap(),
        bullets
            .text_field_offset_rect(plain, 24, TextAffinity::Downstream)
            .unwrap(),
    );
}

#[test]
fn masking_a_retained_field_masks_its_accessible_value_without_rewriting_the_model() {
    let controller = TextEditingController::with_text("é👩‍💻");
    let describe = |obscure| {
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .obscure_text(obscure)
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(describe(false)).unwrap();
    let size = Constraints::tight(Size::new(240., 50.));
    tree.layout(size).unwrap();
    tree.update_semantics();
    let id = tree.semantic_node_for_element(root).unwrap();
    for (obscured, value) in [(true, "••••••"), (false, "é👩‍💻")] {
        tree.update(root, describe(obscured)).unwrap();
        tree.layout(size).unwrap();
        tree.update_semantics();
        assert_eq!(tree.semantic_node_for_element(root), Some(id));
        let node = tree.semantics().node(id).unwrap();
        assert_eq!(node.state.obscured, obscured);
        assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
        assert!(node.state.focusable);
        assert_eq!(node.value.as_deref(), Some(value));
        assert_eq!(controller.text(), "é👩‍💻");
    }
}
