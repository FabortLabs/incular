use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_text::{
    SelectionChangedCause, TextAffinity, TextEditingController, TextEditingValue, TextRange,
    TextSelection,
};
use incular_widgets::{EditableText, TextDirection, TextInputFormatter};

#[derive(Clone)]
struct Counter(Rc<Cell<usize>>);

impl TextInputFormatter for Counter {
    fn format_edit_update(
        &self,
        _: &TextEditingValue,
        next: &TextEditingValue,
    ) -> TextEditingValue {
        self.0.set(self.0.get() + 1);
        next.clone()
    }
}

fn limits() -> Constraints {
    Constraints::loose(Size::new(280.0, 100.0))
}

#[test]
fn replacement_uses_formatters_and_adjusts_marked_text_and_directional_selection() {
    let controller = TextEditingController::from_value(
        TextEditingValue::new("tset next")
            .with_selection(TextSelection::new(4, 0))
            .with_composing(Some(TextRange::new(0, 4))),
    );
    controller.apply_autofill_value(controller.value(), true, TextAffinity::Upstream, true);
    let formats = Rc::new(Cell::new(0));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .input_formatter(Counter(formats.clone()))
            .on_changed({
                let changes = changes.clone();
                move |text| changes.borrow_mut().push(text.to_owned())
            })
            .into(),
    )
    .unwrap();
    runtime.run_frame(limits()).unwrap();
    let field = runtime.tree().root().unwrap();
    assert_eq!(runtime.focused_element(), None);
    let word = TextRange::new(0, 4);
    let toolbar = SelectionChangedCause::Toolbar;

    assert!(runtime.replace_text(field, word, "test", toolbar));
    assert_eq!(runtime.focused_element(), None);
    assert_eq!(controller.text(), "test next");
    assert_eq!(controller.selection(), TextSelection::new(4, 0));
    assert_eq!(controller.caret_affinity(), TextAffinity::Upstream);
    assert!(controller.selection_is_directional());
    assert_eq!(controller.composing(), Some(word));

    assert!(runtime.replace_text(field, word, "ok", toolbar));
    runtime.run_frame(limits()).unwrap();
    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(controller.text(), "ok next");
    assert_eq!(controller.selection(), TextSelection::new(2, 0));
    assert_eq!(controller.caret_affinity(), TextAffinity::Downstream);
    assert!(!controller.selection_is_directional());
    assert_eq!(controller.composing(), Some(TextRange::new(0, 2)));
    assert_eq!(&*changes.borrow(), &["test next", "ok next"]);
    assert_eq!(formats.get(), 2);

    assert!(runtime.replace_text(field, TextRange::new(0, 2), "ok", toolbar));
    assert_eq!(formats.get(), 2);
    assert_eq!(changes.borrow().len(), 2);
}

#[test]
fn identical_replacement_focuses_but_invalid_ranges_and_readonly_never_edit() {
    let text = TextEditingController::with_text("éx");
    let formats = Rc::new(Cell::new(0));
    let field = |read_only| {
        EditableText::new(text.clone())
            .text_direction(TextDirection::Ltr)
            .read_only(read_only)
            .input_formatter(Counter(formats.clone()))
            .into()
    };
    let mut runtime = Runtime::new(field(false)).unwrap();
    runtime.run_frame(limits()).unwrap();
    let root = runtime.tree().root().unwrap();
    let toolbar = SelectionChangedCause::Toolbar;
    assert!(!runtime.replace_text(root, TextRange::new(1, 2), "q", toolbar));
    assert_eq!(runtime.focused_element(), None);
    assert_eq!(formats.get(), 0);
    assert!(runtime.replace_text(root, TextRange::new(0, 2), "é", toolbar));
    assert_eq!(runtime.focused_element(), Some(root));
    assert_eq!(formats.get(), 0);
    assert_eq!(text.text(), "éx");

    let mut readonly = Runtime::new(field(true)).unwrap();
    readonly.run_frame(limits()).unwrap();
    let root = readonly.tree().root().unwrap();
    assert!(!readonly.replace_text(root, TextRange::new(0, 2), "q", toolbar));
    assert_eq!(readonly.focused_element(), None);
    assert_eq!(formats.get(), 0);
    assert_eq!(text.text(), "éx");
}
