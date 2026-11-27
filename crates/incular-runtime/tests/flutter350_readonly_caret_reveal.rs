use std::time::Duration;

use incular_config::{Constraints, TextDirection};
use incular_core::{Rect, Size, time::Instant};
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextSelection, TextStyle};
use incular_widgets::{EditableText, FocusNode, ListView, ScrollController, SizedBox, Widget};

const EDITOR: u64 = 919;

fn field(text: &TextEditingController, focus: &FocusNode, read_only: bool) -> EditableText {
    EditableText::new(text.clone())
        .focus_node(focus.clone())
        .text_direction(TextDirection::Ltr)
        .style(TextStyle::new().font_size(20.).line_height_multiplier(1.))
        .select_all_on_focus(false)
        .read_only(read_only)
}

fn viewport() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn caret(runtime: &Runtime, id: incular_widgets::internal::ElementId) -> Rect {
    runtime
        .tree()
        .text_field_input_geometry(id)
        .unwrap()
        .caret_rect
}

fn on_screen(rect: Rect) -> bool {
    rect.origin.x >= 0.
        && rect.origin.y >= 0.
        && rect.origin.x + rect.size.width <= 800.
        && rect.origin.y + rect.size.height <= 600.
}

#[test]
fn focus_reveals_the_caret_only_for_the_editable_field() {
    for read_only in [false, true] {
        let text = TextEditingController::with_text("a".repeat(100));
        text.set_selection(TextSelection::collapsed(100));
        let focus = FocusNode::new();
        let outer = ScrollController::new();
        let field = Widget::from(field(&text, &focus, read_only)).with_key(EDITOR);
        let list =
            ListView::new([SizedBox::new().height(599.).into(), field]).controller(outer.clone());
        let mut runtime = Runtime::new(list.into()).unwrap();
        runtime.run_frame(viewport()).unwrap();
        focus.request_focus();
        for _ in 0..4 {
            runtime.run_frame(viewport()).unwrap();
        }
        let id = runtime
            .focused_element()
            .expect("both fields can receive focus");
        assert_eq!(runtime.tree().element_with_key(&EDITOR.into()), Some(id));
        assert!(focus.has_primary_focus());
        assert_eq!(on_screen(caret(&runtime, id)), !read_only);
        assert_eq!(outer.offset() > 0., !read_only);
        let inner = runtime.tree().text_field_scroll_offset(id).unwrap();
        assert_eq!(inner.x > 0., !read_only);
    }
}

#[test]
fn a_caret_repaint_does_not_undo_explicit_horizontal_scrolling() {
    for read_only in [false, true] {
        let text = TextEditingController::with_text("a".repeat(100));
        text.set_selection(TextSelection::collapsed(0));
        let focus = FocusNode::new();
        let mut runtime = Runtime::new(field(&text, &focus, read_only).into()).unwrap();
        let start = Instant::now();
        runtime.run_frame_at(viewport(), start).unwrap();
        let id = runtime.tree().root().unwrap();
        assert!(on_screen(caret(&runtime, id)));
        assert_eq!(runtime.tree().text_field_scroll_offset(id).unwrap().x, 0.);

        text.set_text("a".repeat(101));
        text.set_selection(TextSelection::collapsed(0));
        runtime.run_frame_at(viewport(), start).unwrap();
        assert!(on_screen(caret(&runtime, id)));
        assert_eq!(runtime.tree().text_field_scroll_offset(id).unwrap().x, 0.);

        // Also test while focused, crossing both edges of a real blink cycle.
        focus.request_focus();
        runtime.run_frame_at(viewport(), start).unwrap();
        assert!(
            runtime
                .tree_mut()
                .text_field_scroll_to_document_boundary(id, true)
        );
        let before = runtime.tree().text_field_scroll_offset(id).unwrap().x;
        assert!(before > 100.);
        for millis in [0, 500, 1000, 1500] {
            runtime
                .run_frame_at(viewport(), start + Duration::from_millis(millis))
                .unwrap();
            assert_eq!(
                runtime.tree().text_field_scroll_offset(id).unwrap().x,
                before
            );
            assert!(!on_screen(caret(&runtime, id)));
        }
    }
}
