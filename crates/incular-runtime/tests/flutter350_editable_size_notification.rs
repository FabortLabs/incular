use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{PlatformEvent, TextInputState};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{
    Align, EditableText, NotificationListener, SizeChangedLayoutNotification, Widget, internal::Key,
};

#[test]
fn multiline_editor_notifies_an_ancestor_once_when_newline_grows_its_box() {
    let changes = Rc::new(Cell::new(0));
    let controller = TextEditingController::new();
    let editor = Widget::from(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .min_lines(Some(1))
            .max_lines(Some(3)),
    )
    .with_key("editor");
    let root = Align::new(
        incular_config::Alignment::TOP_LEFT,
        NotificationListener::new(editor).on({
            let changes = changes.clone();
            move |_: &SizeChangedLayoutNotification| {
                changes.set(changes.get() + 1);
                false
            }
        }),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let viewport = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(viewport).unwrap();
    assert_eq!(changes.get(), 0);
    let field = runtime
        .tree()
        .element_with_key(&Key::from("editor"))
        .unwrap();
    let initial = runtime.tree().element_bounds(field).unwrap().size;
    assert!(runtime.request_text_input_focus_for(field));
    runtime.run_frame(viewport).unwrap();
    assert_eq!(changes.get(), 0);

    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "\n".into(),
        selection_start: 1,
        selection_end: 1,
        composing: None,
    }));
    runtime.run_frame(viewport).unwrap();
    assert_eq!(controller.text(), "\n");
    assert_eq!(changes.get(), 1);
    let grown = runtime.tree().element_bounds(field).unwrap().size;
    assert_eq!(grown.width, initial.width);
    assert!(grown.height > initial.height);

    runtime.run_frame(viewport).unwrap();
    assert_eq!(changes.get(), 1);
}
