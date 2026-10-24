//! Flutter's `ToolbarOptions` and `EditableText`'s defaults and legacy
//! `cutEnabled`, `copyEnabled`, `pasteEnabled` and `selectAllEnabled`.

use incular_text::TextEditingController;
use incular_widgets::{EditableText, ToolbarOptions};

fn field(read_only: bool, obscure_text: bool) -> EditableText {
    EditableText::new(TextEditingController::new())
        .read_only(read_only)
        .obscure_text(obscure_text)
}

#[test]
fn defaults_follow_read_only_and_obscured() {
    assert_eq!(ToolbarOptions::default(), ToolbarOptions::EMPTY);
    let all = ToolbarOptions {
        copy: true,
        cut: true,
        paste: true,
        select_all: true,
    };
    assert_eq!(field(false, false).resolved_toolbar_options(), all);
    assert_eq!(
        field(true, false).resolved_toolbar_options(),
        ToolbarOptions {
            copy: true,
            select_all: true,
            ..ToolbarOptions::EMPTY
        }
    );
    assert_eq!(
        field(false, true).resolved_toolbar_options(),
        ToolbarOptions {
            paste: true,
            select_all: true,
            ..ToolbarOptions::EMPTY
        }
    );
    assert_eq!(
        field(true, true).resolved_toolbar_options(),
        ToolbarOptions::EMPTY
    );
}

#[test]
fn enabled_actions_also_check_the_field() {
    // Explicit options still cannot cut or copy an obscured field or paste
    // into a read-only one.
    let all = ToolbarOptions {
        copy: true,
        cut: true,
        paste: true,
        select_all: true,
    };
    let obscured = field(false, true).toolbar_options(all);
    assert!(!obscured.cut_enabled());
    assert!(!obscured.copy_enabled());
    assert!(obscured.paste_enabled());
    assert!(obscured.select_all_enabled());
    let read_only = field(true, false).toolbar_options(all);
    assert!(!read_only.cut_enabled());
    assert!(read_only.copy_enabled());
    assert!(!read_only.paste_enabled());
    assert!(
        !field(false, false)
            .toolbar_options(all)
            .enable_interactive_selection(false)
            .select_all_enabled()
    );
}
