use incular_config::{Constraints, TextDirection};
use incular_core::{Size, time::Instant};
use incular_platform::{PlatformEvent, TextInputState};
use incular_runtime::Runtime;
use incular_text::{
    TextAffinity, TextEditingController, TextEditingValue, TextSelection, TextStyle,
};
use incular_widgets::{
    EditableText, FocusNode, ListView, ScrollController, SizedBox, TextInputFormatter, Widget,
};

const EDITOR: u64 = 926;

struct Reject;
impl TextInputFormatter for Reject {
    fn format_edit_update(&self, old: &TextEditingValue, _: &TextEditingValue) -> TextEditingValue {
        old.clone()
    }
}

fn frame(runtime: &mut Runtime) {
    let viewport = Constraints::tight(Size::new(800., 600.));
    for _ in 0..3 {
        runtime.run_frame(viewport).unwrap();
    }
}

fn selection_origin(read_only: bool, keyboard: bool) {
    let value = "a".repeat(100);
    let controller = TextEditingController::with_text(&value);
    controller.set_selection(TextSelection::collapsed(80));
    let focus = FocusNode::new();
    let outer = ScrollController::new();
    let build = |reject| {
        let mut field = EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::new().font_size(20.).line_height_multiplier(1.))
            .select_all_on_focus(false)
            .read_only(read_only);
        if reject {
            field = field.input_formatter(Reject);
        }
        Widget::from(field).with_key(EDITOR)
    };
    let list = ListView::new([SizedBox::new().height(599.).into(), build(false)])
        .controller(outer.clone());
    let mut runtime = Runtime::new(list.into()).unwrap();
    frame(&mut runtime);
    focus.request_focus();
    frame(&mut runtime);
    let field = runtime.focused_element().unwrap();

    for (rejected, next) in [(false, 90), (true, 100)] {
        if rejected {
            runtime.schedule_update(field, build(true)).unwrap();
            frame(&mut runtime);
        }
        let _ = outer.jump_to(0.);
        let _ = runtime
            .tree_mut()
            .text_field_scroll_to_document_boundary(field, false);
        frame(&mut runtime);
        assert_eq!(outer.offset(), 0.);
        assert_eq!(
            runtime.tree().text_field_scroll_offset(field).unwrap().x,
            0.
        );

        if keyboard {
            let _ =
                runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
                    text: value.clone(),
                    selection_start: next,
                    selection_end: next,
                    composing: None,
                }));
        } else {
            assert!(runtime.tree_mut().text_field_select(
                field,
                TextSelection::collapsed(next),
                TextAffinity::Downstream,
                Instant::now()
            ));
        }
        frame(&mut runtime);
        let accepted = !keyboard || !read_only;
        assert_eq!(controller.text(), value);
        assert_eq!(
            controller.selection(),
            TextSelection::collapsed(if accepted { next } else { 80 })
        );
        assert_eq!(outer.offset() > 0., accepted);
        assert_eq!(
            runtime.tree().text_field_scroll_offset(field).unwrap().x > 0.,
            accepted
        );
        assert!(focus.has_primary_focus());
    }
}

#[test]
fn native_keyboard_selection_reveals_only_the_editable_field() {
    for read_only in [false, true] {
        selection_origin(read_only, true);
    }
}

#[test]
fn text_selection_delegate_reveals_even_a_readonly_field() {
    for read_only in [false, true] {
        selection_origin(read_only, false);
    }
}
