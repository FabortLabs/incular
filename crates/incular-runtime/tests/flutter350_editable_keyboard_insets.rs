use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{
    Column, EditableText, FocusNode, ScrollController, SingleChildScrollView, SizedBox, Widget,
};

fn frame(runtime: &mut Runtime, height: f32) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., height)))
        .unwrap();
}

fn keyboard_inset(runtime: &mut Runtime, bottom: f32) {
    let mut environment = runtime.environment();
    environment.view_insets = EdgeInsets {
        bottom,
        ..EdgeInsets::ZERO
    };
    assert!(runtime.set_environment(environment));
}

fn mount(
    spacers: &[f32],
    text: &str,
) -> (Runtime, FocusNode, TextEditingController, ScrollController) {
    let node = FocusNode::new();
    let editing = TextEditingController::with_text(text);
    let scroll = ScrollController::new();
    let mut children: Vec<_> = spacers
        .iter()
        .map(|&size| Widget::from(SizedBox::new().height(size)))
        .collect();
    children.push(
        SizedBox::new()
            .height(20.)
            .child(
                EditableText::new(editing.clone())
                    .focus_node(node.clone())
                    .text_direction(TextDirection::Ltr),
            )
            .into(),
    );
    let root = SingleChildScrollView::new(Column::new(children)).controller(scroll.clone());
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime, 600.);
    node.request_focus();
    frame(&mut runtime, 600.);
    frame(&mut runtime, 600.);
    assert!(node.has_primary_focus());
    assert!(scroll.jump_to(0.));
    frame(&mut runtime, 600.);
    assert_eq!(scroll.offset(), 0.);
    (runtime, node, editing, scroll)
}

#[test]
fn keyboard_inset_growth_reveals_the_caret_immediately_without_a_selection_change() {
    let (mut runtime, node, editing, scroll) = mount(&[1200.; 5], "");
    let original = editing.selection();
    keyboard_inset(&mut runtime, 500.);
    frame(&mut runtime, 100.);
    assert_eq!(scroll.offset(), 5920., "first frame must jump to the end");
    assert_eq!(scroll.offset(), scroll.max_offset());
    assert!(node.has_primary_focus());
    assert_eq!(editing.selection(), original);

    assert!(scroll.jump_to(0.));
    frame(&mut runtime, 100.);
    keyboard_inset(&mut runtime, 400.);
    frame(&mut runtime, 200.);
    assert_eq!(
        scroll.offset(),
        0.,
        "a receding keyboard does not reveal again"
    );
}

#[test]
fn selecting_text_while_the_keyboard_appears_reveals_the_extent_on_the_first_frame() {
    let (mut runtime, node, editing, scroll) = mount(&[1000.], "I love flutter");
    keyboard_inset(&mut runtime, 500.);
    editing.set_selection(TextSelection {
        base: 0,
        extent: editing.text().len(),
    });
    frame(&mut runtime, 100.);
    assert!(
        scroll.offset() > 900.,
        "selection reveal must not animate from zero; got {}",
        scroll.offset()
    );
    assert!(node.has_primary_focus());
    assert_eq!(editing.selection().base, 0);
    assert_eq!(editing.selection().extent, "I love flutter".len());
}
