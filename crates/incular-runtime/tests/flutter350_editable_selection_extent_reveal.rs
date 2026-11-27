use incular_config::{Axis, Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextSelection, TextStyle};
use incular_widgets::{EditableText, FocusNode, ScrollController, SingleChildScrollView};

#[test]
fn horizontal_parent_reveals_the_selected_glyph_not_the_wider_caret() {
    let Some(font) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    const AHEM: &str = "Pinned selection extent Ahem";
    incular_text::register_application_font_as(AHEM, std::fs::read(font).unwrap());
    let editing = TextEditingController::with_text("AAAAAAAAAAAAAAAAAAAA");
    editing.set_selection(TextSelection::collapsed(0));
    let focus = FocusNode::new();
    let scroll = ScrollController::new();
    let editor = EditableText::new(editing.clone())
        .autofocus(true)
        .select_all_on_focus(false)
        .focus_node(focus.clone())
        .text_direction(TextDirection::Ltr)
        .style(
            TextStyle::new()
                .font_family(AHEM)
                .font_size(10.)
                .height(Some(1.)),
        )
        .size(Size::new(220., 40.));
    let root = SingleChildScrollView::new(editor)
        .scroll_direction(Axis::Horizontal)
        .controller(scroll.clone());
    let mut runtime = Runtime::new(root.into()).unwrap();
    let frame = |runtime: &mut Runtime| {
        runtime
            .run_frame(Constraints::tight(Size::new(100., 40.)))
            .unwrap();
    };
    frame(&mut runtime);
    assert_eq!(scroll.offset(), 0.);
    assert!(focus.has_primary_focus());

    editing.set_selection(TextSelection::new(0, 18));
    frame(&mut runtime);
    assert_eq!(
        scroll.offset(),
        0.,
        "the selection box already covers the viewport"
    );

    editing.set_selection(TextSelection::new(16, 18));
    frame(&mut runtime);
    assert_eq!(
        scroll.offset(),
        100.,
        "painted glyph ends at 180 plus padding 20"
    );
    frame(&mut runtime);
    editing.set_selection(TextSelection::new(2, 0));
    frame(&mut runtime);
    assert_eq!(scroll.offset(), 0.);
    assert!(focus.has_primary_focus());
}
