use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{MemoryClipboard, TransferReadRequest};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{
    ContextMenuButtonItem, ContextMenuButtonType, EditableText, ToolbarOptions, internal::ElementId,
};

const SOURCE: &str = "TEXT";

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 100.)))
        .unwrap();
}

fn create(
    options: ToolbarOptions,
    read_only: bool,
    obscure: bool,
) -> (Runtime, ElementId, TextEditingController) {
    let controller = TextEditingController::with_text(SOURCE);
    let input = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .read_only(read_only)
        .obscure_text(obscure)
        .toolbar_options(options);
    let mut runtime = Runtime::new(input.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime.set_clipboard(Box::<MemoryClipboard>::default());
    frame(&mut runtime);
    let field = runtime.tree().root().unwrap();
    assert!(runtime.focused_element().is_none());
    (runtime, field, controller)
}

fn checked_clipboard(runtime: &mut Runtime) -> String {
    runtime
        .read_clipboard(TransferReadRequest::all())
        .unwrap()
        .plain_text_value()
        .unwrap()
        .unwrap()
        .to_string()
}

fn action(list: Vec<ContextMenuButtonItem>, kind: ContextMenuButtonType) -> ContextMenuButtonItem {
    let [button] = <[ContextMenuButtonItem; 1]>::try_from(list)
        .ok()
        .expect("one explicit option");
    assert_eq!(button.button_type(), kind);
    button
}

#[test]
fn an_empty_toolbar_options_record_requests_the_fallback_not_an_empty_custom_list() {
    let (mut runtime, field, _) = create(ToolbarOptions::EMPTY, false, false);
    assert!(
        runtime
            .editable_button_items_for_toolbar_options(field)
            .is_none()
    );
}

#[test]
fn explicitly_chosen_but_disabled_actions_return_empty_collections() {
    let cases = [
        (
            ToolbarOptions {
                cut: true,
                ..ToolbarOptions::EMPTY
            },
            true,
            false,
        ),
        (
            ToolbarOptions {
                copy: true,
                ..ToolbarOptions::EMPTY
            },
            false,
            true,
        ),
        (
            ToolbarOptions {
                paste: true,
                ..ToolbarOptions::EMPTY
            },
            true,
            false,
        ),
        (
            ToolbarOptions {
                select_all: true,
                ..ToolbarOptions::EMPTY
            },
            true,
            true,
        ),
    ];
    for (option, read_only, obscure) in cases {
        let (mut runtime, field, text) = create(option, read_only, obscure);
        assert!(
            runtime
                .editable_button_items_for_toolbar_options(field)
                .unwrap()
                .is_empty(),
            "{option:?}"
        );
        assert!(runtime.focused_element().is_none());
        assert_eq!(text.text(), SOURCE);
    }
}

#[test]
fn explicit_cut_and_copy_offer_only_the_requested_action_and_use_the_clipboard() {
    for (kind, options, expected) in [
        (
            ContextMenuButtonType::Cut,
            ToolbarOptions {
                cut: true,
                ..ToolbarOptions::EMPTY
            },
            "",
        ),
        (
            ContextMenuButtonType::Copy,
            ToolbarOptions {
                copy: true,
                ..ToolbarOptions::EMPTY
            },
            SOURCE,
        ),
    ] {
        let (mut runtime, field, text) = create(options, false, false);
        text.set_selection(TextSelection::new(0, SOURCE.len()));
        frame(&mut runtime);
        let item = action(
            runtime
                .editable_button_items_for_toolbar_options(field)
                .unwrap(),
            kind,
        );
        assert!(
            runtime.focused_element().is_none(),
            "querying must not focus"
        );
        item.press();
        frame(&mut runtime);
        assert_eq!(text.text(), expected);
        assert_eq!(checked_clipboard(&mut runtime), SOURCE);
    }
}

#[test]
fn explicit_paste_is_available_before_clipboard_data_arrives_and_inserts_at_the_caret() {
    let (mut runtime, field, text) = create(
        ToolbarOptions {
            paste: true,
            ..ToolbarOptions::EMPTY
        },
        false,
        false,
    );
    text.set_selection(TextSelection::collapsed(SOURCE.len()));
    frame(&mut runtime);
    let item = action(
        runtime
            .editable_button_items_for_toolbar_options(field)
            .unwrap(),
        ContextMenuButtonType::Paste,
    );
    assert!(runtime.focused_element().is_none());
    runtime.set_clipboard_text(SOURCE);
    item.press();
    frame(&mut runtime);
    assert_eq!(text.text(), SOURCE.repeat(2));
    assert_eq!(text.selection(), TextSelection::collapsed(8));
}

#[test]
fn explicit_select_all_returns_one_action_which_selects_without_changing_text() {
    let (mut runtime, field, text) = create(
        ToolbarOptions {
            select_all: true,
            ..ToolbarOptions::EMPTY
        },
        false,
        false,
    );
    let item = action(
        runtime
            .editable_button_items_for_toolbar_options(field)
            .unwrap(),
        ContextMenuButtonType::SelectAll,
    );
    assert!(runtime.focused_element().is_none());
    item.press();
    frame(&mut runtime);
    assert_eq!(text.text(), SOURCE);
    assert_eq!(text.selected_text(), SOURCE);
}
