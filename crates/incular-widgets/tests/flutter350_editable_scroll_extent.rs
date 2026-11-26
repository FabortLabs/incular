use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_text::{TextEditingController, TextSelection, TextStyle, register_application_font_as};
use incular_widgets::{EditableText, internal::WidgetTree};

#[test]
fn a_two_line_editor_reports_the_full_wrapped_mixed_script_scroll_extent() {
    const AHEM: &str = "Pinned editable extent Ahem";
    const CJK: &str = "Audit editable extent Noto CJK";
    let Some(ahem) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let Some(cjk) = std::env::var_os("INCULAR_FLUTTER_CJK_FONT_PATH") else {
        return;
    };
    register_application_font_as(AHEM, std::fs::read(ahem).expect("read pinned Ahem"));
    register_application_font_as(CJK, std::fs::read(cjk).expect("read CJK fixture"));
    let controller = TextEditingController::with_text(concat!(
        "撒地方加咖啡哈金凤凰卡号方式剪坏算法发挥福建垃\n",
        "asfjafjajfjaslfjaskjflasjfksajf撒分开建安路口附近拉设\n",
        "计费可使肌肤撒附近埃里克圾房卡设计费\"",
    ));
    let caret = TextSelection::collapsed(controller.text().char_indices().nth(4).unwrap().0);
    controller.set_selection(caret);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .style(
                    TextStyle::new()
                        .font_size(10.)
                        .font_family(AHEM)
                        .fallback_families([CJK])
                        .line_height_multiplier(1.),
                )
                .text_direction(TextDirection::Ltr)
                .cursor_width(1.)
                .max_lines(Some(2))
                .into(),
        )
        .unwrap();
    let render = tree.render_id(root);
    assert_eq!(tree.editable_max_scroll_extent(root), None);
    for (width, extent) in [
        (100., 90.),
        (150., 50.),
        (200., 40.),
        (500., 10.),
        (1000., 10.),
    ] {
        tree.layout(Constraints::loose(Size::new(width, 1000.)))
            .unwrap();
        assert_eq!(
            tree.element_bounds(root).unwrap().size,
            Size::new(width, 20.)
        );
        assert_eq!(
            tree.editable_max_scroll_extent(root),
            Some(extent),
            "width {width}"
        );
        assert_eq!(tree.render_id(root), render);
    }
    assert!(tree.text_field_scroll_page(root, true));
    assert_eq!(tree.text_field_scroll_offset(root).unwrap().y, 10.);
    assert!(!tree.text_field_scroll_page(root, true));
    assert_eq!(controller.selection(), caret);
}

#[test]
fn a_single_line_extent_agrees_with_page_scroll_and_resets_after_editing() {
    let controller = TextEditingController::with_text("a very long single line");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .into(),
        )
        .unwrap();
    let viewport = Constraints::tight(Size::new(40., 32.));
    tree.layout(viewport).unwrap();
    let extent = tree.editable_max_scroll_extent(root).unwrap();
    assert!(extent > 0.);
    assert!(tree.text_field_scroll_page(root, true));
    assert_eq!(tree.text_field_scroll_offset(root).unwrap().x, extent);
    assert!(!tree.text_field_scroll_page(root, true));
    controller.set_text("");
    assert_eq!(tree.editable_max_scroll_extent(root), None);
    tree.layout(viewport).unwrap();
    let _ = tree.paint();
    assert_eq!(tree.editable_max_scroll_extent(root), Some(0.));
    assert_eq!(tree.text_field_scroll_offset(root).unwrap().x, 0.);
}
