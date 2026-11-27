use incular_platform::{
    CapabilitySupport, DataTransfer, TransferData, TransferFormat, TransferItem,
    TransferReadRequest, TransferRepresentation,
};
use incular_runtime::Runtime;
use incular_widgets::SizedBox;

#[test]
fn runtime_exposes_rich_clipboard_without_text_only_side_channel() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).expect("runtime");
    let capabilities = runtime.clipboard_capabilities();
    assert_eq!(capabilities.read.plain_text, CapabilitySupport::Supported);
    assert_eq!(capabilities.read.html, CapabilitySupport::Supported);

    let transfer = DataTransfer::single(
        TransferItem::new([
            TransferRepresentation::plain_text("plain"),
            TransferRepresentation::html("<b>plain</b>"),
        ])
        .unwrap(),
    );
    let report = runtime.write_clipboard(transfer).expect("memory write");
    assert_eq!(
        report.written,
        vec![TransferFormat::PlainText, TransferFormat::Html]
    );

    let html = runtime
        .read_clipboard(TransferReadRequest::format(TransferFormat::Html))
        .expect("HTML read");
    assert_eq!(
        html.materialize(&TransferFormat::Html).unwrap(),
        Some(TransferData::text("<b>plain</b>"))
    );

    runtime.set_clipboard_text("replacement");
    let plain = runtime
        .read_clipboard(TransferReadRequest::all())
        .expect("plain-text convenience writes rich model");
    assert_eq!(plain.formats(), vec![TransferFormat::PlainText]);
}

struct FailingClipboard {
    read: incular_platform::ClipboardError,
    write: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl incular_platform::Clipboard for FailingClipboard {
    fn capabilities(&self) -> incular_platform::ClipboardCapabilities {
        use incular_platform::MemoryClipboard;
        MemoryClipboard::default().capabilities()
    }

    fn read(
        &mut self,
        request: TransferReadRequest,
    ) -> Result<DataTransfer, incular_platform::ClipboardError> {
        assert_eq!(request.requested_formats(), [TransferFormat::PlainText]);
        Err(self.read.clone())
    }

    fn write(
        &mut self,
        transfer: DataTransfer,
    ) -> Result<incular_platform::ClipboardWriteReport, incular_platform::ClipboardError> {
        self.write
            .borrow_mut()
            .push(transfer.plain_text_value().unwrap().unwrap().to_string());
        Err(incular_platform::ClipboardError::Backend(
            "Clipboard error".into(),
        ))
    }
}

#[test]
fn copy_reports_the_backend_error_with_context_without_changing_the_editor() {
    use incular_config::Constraints;
    use incular_core::Size;
    use incular_platform::ClipboardError;
    use incular_runtime::EditorClipboardOperation as Operation;
    use incular_text::{SelectionChangedCause, TextEditingController, TextSelection};
    use incular_widgets::{EditableText, TextDirection};
    use std::{cell::RefCell, rc::Rc};

    let controller = TextEditingController::with_text("test data");
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .into(),
    )
    .unwrap();
    let writes = Rc::new(RefCell::new(Vec::new()));
    runtime.set_clipboard(Box::new(FailingClipboard {
        read: ClipboardError::Unavailable,
        write: writes.clone(),
    }));
    let reports = Rc::new(RefCell::new(Vec::new()));
    let seen = reports.clone();
    runtime.observe_editor_clipboard_error(move |report| seen.borrow_mut().push(report.clone()));
    runtime
        .run_frame(Constraints::tight(Size::new(300., 40.)))
        .unwrap();
    let field = runtime.tree().root().unwrap();
    let original = TextSelection::new(0, 4);
    controller.set_selection(original);

    assert!(runtime.copy_text_selection(field, SelectionChangedCause::Toolbar));
    assert_eq!(&*writes.borrow(), &["test"]);
    let reports = reports.borrow();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].field, field);
    assert_eq!(reports[0].operation, Operation::Copy);
    assert!(
        reports[0]
            .operation
            .context()
            .contains("while copying selection")
    );
    assert_eq!(
        reports[0].error,
        ClipboardError::Backend("Clipboard error".into())
    );
    assert_eq!(controller.text(), "test data");
    assert_eq!(controller.selection(), original);
    assert!(runtime.focused_element().is_none());
}

#[test]
fn paste_reports_backend_failure_but_not_missing_clipboard_content() {
    use incular_config::Constraints;
    use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
    use incular_platform::ClipboardError;
    use incular_runtime::{EditorClipboardOperation as Operation, TextShortcutPlatform};
    use incular_text::{TextEditingController, TextSelection};
    use incular_widgets::{EditableText, FocusNode, TextDirection};
    use std::{cell::RefCell, rc::Rc};

    let controller = TextEditingController::with_text("test data");
    let focus = FocusNode::new();
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .focus_node(focus.clone())
            .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    let bounds = Constraints::tight(Size::new(300., 40.));
    runtime.run_frame(bounds).unwrap();
    focus.request_focus();
    runtime.run_frame(bounds).unwrap();
    let field = runtime.focused_element().unwrap();
    let selection = TextSelection::new(0, 4);
    controller.set_selection(selection);

    let reports = Rc::new(RefCell::new(Vec::new()));
    let seen = reports.clone();
    runtime.observe_editor_clipboard_error(move |report| seen.borrow_mut().push(report.clone()));
    let failure = ClipboardError::Backend("Failed to read clipboard".into());
    for read in [ClipboardError::Unavailable, failure.clone()] {
        runtime.set_clipboard(Box::new(FailingClipboard {
            read,
            write: Default::default(),
        }));
        let mut event =
            KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), Code::KeyV);
        event.modifiers = Modifiers::CONTROL;
        runtime.dispatch_keyboard_event(event);
        runtime.run_frame(bounds).unwrap();
        assert_eq!(controller.text(), "test data");
        assert_eq!(controller.selection(), selection);
        assert_eq!(runtime.focused_element(), Some(field));
    }
    let report = reports.borrow();
    assert_eq!(report.len(), 1);
    assert_eq!(report[0].operation, Operation::Paste);
    assert_eq!(report[0].field, field);
    assert_eq!(report[0].error, failure);
}
