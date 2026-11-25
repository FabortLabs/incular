//! Real frame/focus dispatch across the retained Table row and cell wrappers.
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::Role;
use incular_text::TextEditingController;
use incular_widgets::{EditableText, FocusNode, Table};

#[test]
fn a_table_reuses_its_semantic_cell_wrapper_when_an_editor_gains_focus() {
    let focus = FocusNode::new();
    let editor = EditableText::new(TextEditingController::with_text("cell"))
        .text_direction(TextDirection::Ltr)
        .focus_node(focus.clone())
        .size(Size::new(180., 40.));
    let mut runtime = Runtime::new(Table::new(1, [editor]).into()).unwrap();
    let frame = |runtime: &mut Runtime| {
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    };

    let ids = |runtime: &Runtime| {
        let semantics = runtime.tree().semantics();
        let table = semantics.node(semantics.root().unwrap()).unwrap();
        assert_eq!(table.role, Role::Table);
        let row = semantics.node(table.children[0]).unwrap();
        assert_eq!(row.role, Role::Row);
        let cell = semantics.node(row.children[0]).unwrap();
        assert_eq!(cell.role, Role::Cell);
        let editor = semantics.node(cell.children[0]).unwrap();
        assert_eq!(editor.role, Role::TextField);
        assert_eq!(editor.value.as_deref(), Some("cell"));
        (row.id, cell.id, editor.id, editor.state.focused)
    };

    frame(&mut runtime);
    let (row, cell, editor, focused) = ids(&runtime);
    assert!(!focused);

    focus.request_focus();
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    assert_eq!(ids(&runtime), (row, cell, editor, true));
}
