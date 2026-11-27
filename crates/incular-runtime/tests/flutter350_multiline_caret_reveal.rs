use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{PlatformEvent, TextInputState};
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextSelection, TextStyle};
use incular_widgets::{
    Directionality, EditableText, ListView, PageController, PageView, ScrollController, SizedBox,
    Widget,
};

const KEY: u64 = 907;

fn viewport() -> Constraints {
    Constraints::tight(Size::new(800., 300.))
}

fn frame(runtime: &mut Runtime) {
    for _ in 0..3 {
        runtime.run_frame(viewport()).unwrap();
    }
}

fn editor(controller: &TextEditingController) -> EditableText {
    EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .select_all_on_focus(false)
        .style(TextStyle::new().font_size(14.).line_height_multiplier(1.))
}

fn open_keyboard(runtime: &mut Runtime) -> incular_widgets::internal::ElementId {
    let field = runtime.tree().element_with_key(&KEY.into()).unwrap();
    assert!(runtime.request_text_input_focus_for(field));
    frame(runtime);
    assert_eq!(runtime.focused_element(), Some(field));
    field
}

fn enter(runtime: &mut Runtime, text: &str, base: usize, extent: usize) {
    runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: text.into(),
        selection_start: base,
        selection_end: extent,
        composing: None,
    }));
}

#[test]
fn editing_on_the_middle_page_keeps_the_page_still() {
    let controller = TextEditingController::new();
    let pages = PageController::new().with_initial_page(1);
    let field = Widget::from(editor(&controller)).with_key(KEY);
    let page_view = PageView::new([SizedBox::expand().into(), field, SizedBox::expand().into()])
        .controller(pages.clone());
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, page_view).into()).unwrap();
    frame(&mut runtime);
    open_keyboard(&mut runtime);
    assert_eq!(pages.page(), Some(1.));
    assert_eq!(controller.text(), "");
    enter(&mut runtime, "H", 1, 1);
    let (_, stats) = runtime.run_frame(viewport()).unwrap();
    assert_eq!(stats.active_animations, 0);
    assert!(!stats.requested_another_frame);
    assert_eq!(pages.page(), Some(1.));
    assert_eq!(controller.text(), "H");
}

fn mount_tall_editor(controller: &TextEditingController) -> (Runtime, ScrollController) {
    let scroll = ScrollController::new();
    let field = Widget::from(editor(controller).max_lines(None)).with_key(KEY);
    let list = ListView::new([field]).controller(scroll.clone());
    let mut runtime = Runtime::new(list.into()).unwrap();
    frame(&mut runtime);
    let field = open_keyboard(&mut runtime);
    let _ = scroll.jump_to(0.);
    frame(&mut runtime);
    assert!(runtime.tree().element_bounds(field).unwrap().size.height > 500.);
    assert_eq!(scroll.offset(), 0.);
    (runtime, scroll)
}

#[test]
fn editing_the_last_multiline_line_scrolls_the_caret_into_view() {
    let text = format!("Start{}End", "\n".repeat(39));
    let controller = TextEditingController::with_text(&text);
    let (mut runtime, scroll) = mount_tall_editor(&controller);
    let changed = format!("{text} HELLO");
    enter(&mut runtime, &changed, changed.len(), changed.len());
    frame(&mut runtime);
    assert_eq!(controller.text(), changed);
    assert!(scroll.offset() > 0.);
    let editor = runtime.focused_element().unwrap();
    let caret = runtime
        .tree()
        .text_field_input_geometry(editor)
        .unwrap()
        .caret_rect;
    assert!(caret.origin.y + caret.size.height <= 300.);
}

#[test]
fn selection_of_a_hard_break_does_not_return_to_the_old_caret() {
    let text = format!("Start{}End", "\n".repeat(39));
    let controller = TextEditingController::with_text(&text);
    controller.set_selection(TextSelection::collapsed(text.len() - 3));
    let (mut runtime, scroll) = mount_tall_editor(&controller);
    enter(&mut runtime, &text, 26, 27);
    frame(&mut runtime);
    assert_eq!(controller.selection(), TextSelection::new(26, 27));
    assert_eq!(scroll.offset(), 28.);
}
