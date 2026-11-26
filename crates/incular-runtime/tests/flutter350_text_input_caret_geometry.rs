//! Retained editor to native IME geometry, using the pinned Ahem advances.
use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_platform::{TextInputClientId, TextInputCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection, TextStyle, register_application_font_as};
use incular_widgets::{EditableText, FocusNode};

fn runtime(controller: &TextEditingController) -> Option<Runtime> {
    const FAMILY: &str = "Pinned IME caret Ahem";
    let ahem = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")?;
    register_application_font_as(FAMILY, std::fs::read(ahem).expect("read pinned Ahem"));
    let mut result = Runtime::new(
        EditableText::new(controller.clone())
            .autofocus(true)
            .focus_node(FocusNode::new())
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::new().font_family(FAMILY).font_size(14.))
            .into(),
    )
    .unwrap();
    result.set_text_shortcut_platform(TextShortcutPlatform::Android);
    Some(result)
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight_for(Some(1600.), None))
        .unwrap();
}

fn geometry(command: &TextInputCommand) -> Option<(TextInputClientId, Rect)> {
    match command {
        TextInputCommand::SetClient {
            configuration,
            caret_rect,
            ..
        } => Some((configuration.client, *caret_rect)),
        TextInputCommand::SetCaretRect { client, rect } => Some((*client, *rect)),
        _ => None,
    }
}

#[test]
fn forward_and_reverse_selections_anchor_the_native_ime_at_the_same_start() {
    let controller = TextEditingController::with_text("a".repeat(100));
    controller.set_selection(TextSelection::new(10, 30));
    let Some(mut runtime) = runtime(&controller) else {
        return;
    };
    frame(&mut runtime);
    let initial = runtime.take_text_input_commands();
    let (client, caret) = initial.iter().filter_map(geometry).last().unwrap();
    assert_eq!(caret.origin, Offset::new(140., 0.));
    let root = runtime.focused_element().unwrap();
    let visual = runtime
        .tree()
        .text_field_input_geometry(root)
        .unwrap()
        .caret_rect;
    assert_eq!(
        visual.origin,
        Offset::new(420., 0.),
        "visual caret follows the extent"
    );

    controller.set_selection(TextSelection::new(30, 10));
    frame(&mut runtime);
    let reversed = runtime.take_text_input_commands();
    assert!(reversed.iter().any(|command| matches!(command,
        TextInputCommand::Update { client: id, state }
        if *id == client && state.selection_start == 30 && state.selection_end == 10
    )));
    assert!(
        reversed
            .iter()
            .filter_map(geometry)
            .all(|(id, rect)| id == client && rect == caret),
        "reversing a selection must not move the host anchor: {reversed:?}"
    );

    controller.set_selection(TextSelection::new(30, 12));
    frame(&mut runtime);
    let moved = runtime.take_text_input_commands();
    let next = moved.iter().filter_map(geometry).last().unwrap();
    assert_eq!(next.0, client);
    assert_eq!(next.1.origin, Offset::new(168., 0.));
}

#[test]
fn native_ime_caret_has_the_actual_cursor_size_and_only_updates_when_needed() {
    let controller = TextEditingController::with_text("a".repeat(50));
    let Some(mut runtime) = runtime(&controller) else {
        return;
    };
    frame(&mut runtime);
    let commands = runtime.take_text_input_commands();
    let (client, caret) = commands.iter().filter_map(geometry).last().unwrap();
    assert_eq!(
        caret,
        Rect::from_origin_size(Offset::new(700., 0.), Size::new(2., 14.))
    );

    frame(&mut runtime);
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|command| geometry(command).is_some())
    );

    controller.set_selection(TextSelection::collapsed(0));
    frame(&mut runtime);
    let moved = runtime.take_text_input_commands();
    assert_eq!(
        moved.iter().filter_map(geometry).last(),
        Some((
            client,
            Rect::from_origin_size(Offset::ZERO, Size::new(2., 14.))
        )),
    );
}
