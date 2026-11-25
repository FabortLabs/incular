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

#[test]
fn revealing_a_retained_table_row_uses_its_current_bounds() {
    use incular_semantics::{SemanticAction, SemanticActionKind};
    use incular_widgets::{
        Center, Directionality, ScrollController, SingleChildScrollView, SizedBox, Text, Widget,
        internal::Key,
    };

    let controller = ScrollController::new();
    let scene = |first_height, key: &str| {
        let cells = [
            SizedBox::from_size(Size::new(200., first_height)).child(Text::new("first")),
            SizedBox::from_size(Size::new(200., 40.)).child(Text::new("second")),
        ];
        let table = Widget::from(Table::new(1, cells)).with_key(Key::from(key));
        let view = SingleChildScrollView::new(table).controller(controller.clone());
        let fixed = SizedBox::from_size(Size::new(200., 120.)).child(view);
        Directionality::new(TextDirection::Ltr, Center::new(fixed)).into()
    };
    let pump = |runtime: &mut Runtime| {
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    };
    let update = |runtime: &mut Runtime, widget| {
        let root = runtime.tree().root().unwrap();
        runtime.schedule_update(root, widget).unwrap();
        pump(runtime);
    };
    let row = |runtime: &Runtime| {
        let semantics = runtime.tree().semantics();
        let table = semantics
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role == Role::Table)
            .unwrap();
        let row = semantics.node(table.children[1]).unwrap();
        assert_eq!(row.role, Role::Row);
        assert!(row.actions.contains(&SemanticActionKind::ShowOnScreen));
        (row.id, row.bounds.origin.y)
    };

    let mut runtime = Runtime::new(scene(50., "retained")).unwrap();
    pump(&mut runtime);
    let (original, before) = row(&runtime);
    assert_eq!(controller.max_offset(), 0.);

    update(&mut runtime, scene(100., "retained"));
    let (retained, after) = row(&runtime);
    assert_eq!(retained, original);
    assert_eq!(after - before, 50.);
    assert_eq!(controller.max_offset(), 20.);
    assert_eq!(controller.offset(), 0.);
    assert!(runtime.dispatch_semantic_action(retained, SemanticAction::ShowOnScreen));
    pump(&mut runtime);
    assert_eq!(
        controller.offset(),
        20.,
        "reveal the row at its current position"
    );

    assert!(controller.jump_to(0.));
    update(&mut runtime, scene(100., "fresh"));
    let (fresh, _) = row(&runtime);
    assert_ne!(fresh, original);
    assert!(!runtime.dispatch_semantic_action(original, SemanticAction::ShowOnScreen));
    assert!(runtime.dispatch_semantic_action(fresh, SemanticAction::ShowOnScreen));
    pump(&mut runtime);
    assert_eq!(controller.offset(), 20.);
}
