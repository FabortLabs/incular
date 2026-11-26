use incular_config::{Constraints, EdgeInsets, Locale, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_text::{
    TextEditingController, TextRange, TextSelection, TextStyle, register_application_font_as,
};
use incular_widgets::{
    EditableText, SpellCheckCallback, SpellCheckConfiguration, SpellCheckService, SuggestionSpan,
    internal::WidgetTree,
};

fn ahem() -> Option<TextStyle> {
    const FAMILY: &str = "Pinned composing Ahem";
    let path = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")?;
    register_application_font_as(FAMILY, std::fs::read(path).expect("read pinned Ahem"));
    Some(TextStyle::new().font_family(FAMILY))
}

#[test]
fn invalid_collapsed_and_non_rendering_ranges_have_no_composing_bounds() {
    let controller = TextEditingController::with_text("123");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .max_lines(None)
                .into(),
        )
        .unwrap();
    let constraints = Constraints::tight_for(Some(200.), None);
    let range = |start, end| TextRange::new(start, end);
    assert_eq!(tree.editable_composing_rect(root, range(0, 2)), None);
    tree.layout(constraints).unwrap();
    assert!(
        tree.editable_composing_rect(root, range(0, 2))
            .unwrap()
            .size
            .width
            > 0.
    );
    for invalid in [range(usize::MAX, 2), range(2, 1), range(2, 2), range(0, 4)] {
        assert_eq!(tree.editable_composing_rect(root, invalid), None);
    }

    controller.set_text("\u{200e}");
    assert_eq!(tree.editable_composing_rect(root, range(0, 2)), None);
    tree.layout(constraints).unwrap();
    // A full UTF-8 LRM occupies three bytes here and one UTF-16 unit upstream.
    assert_eq!(tree.editable_composing_rect(root, range(0, 3)), None);
    assert_eq!(tree.editable_composing_rect(root, range(0, 1)), None);
}

struct StyleTheEnds;

impl SpellCheckService for StyleTheEnds {
    fn fetch_spell_check_suggestions(&self, _: &Locale, text: &str, respond: SpellCheckCallback) {
        assert_eq!(text, "A".repeat(22));
        respond(Some(vec![
            SuggestionSpan::new(0..1, ["first"]),
            SuggestionSpan::new(21..22, ["last"]),
        ]));
    }
}

#[test]
fn composing_bounds_union_the_tiny_and_normal_styled_runs_of_a_wrapping_editor() {
    let Some(style) = ahem() else { return };
    let controller = TextEditingController::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .max_lines(None)
                .cursor_width(1.)
                .style(style.font_size(10.))
                .spell_check_configuration(
                    SpellCheckConfiguration::new()
                        .spell_check_service(StyleTheEnds)
                        .misspelled_text_style(TextStyle::new().font_size(1.)),
                )
                .into(),
        )
        .unwrap();
    controller.insert_text(&"A".repeat(22));
    // The spell-check widget intentionally leaves the word with the caret
    // unstyled. Move it into the middle so both ends take the small font.
    controller.set_selection(TextSelection::collapsed(11));
    tree.layout(Constraints::tight_for(Some(200.), None))
        .unwrap();
    let first = tree
        .editable_composing_rect(root, TextRange::new(0, 1))
        .unwrap();
    let last = tree
        .editable_composing_rect(root, TextRange::new(21, 22))
        .unwrap();
    let all = tree
        .editable_composing_rect(root, TextRange::new(0, 22))
        .unwrap();
    assert!((first.size.width - 1.).abs() < 0.001, "{first:?}");
    assert!((last.size.width - 1.).abs() < 0.001, "{last:?}");
    assert!(all.size.width > 190. && all.size.width <= 200., "{all:?}");
    assert!(last.origin.y > first.origin.y, "{first:?} / {last:?}");
    assert_eq!(all.origin, first.origin);
    assert_eq!(all.size.height, 20.);
}

#[test]
fn composing_bounds_use_the_actual_laid_out_preedit_and_content_offsets() {
    let Some(style) = ahem() else { return };
    let controller = TextEditingController::with_text("1234");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .style(style.font_size(10.))
                .content_padding(EdgeInsets::all(3.))
                .into(),
        )
        .unwrap();
    let constraints = Constraints::tight(Size::new(100., 20.));
    tree.layout(constraints).unwrap();
    controller.set_selection(TextSelection::new(1, 3));
    controller.set_preedit("ABC", None);
    let preedit = TextRange::new(1, 4);
    assert_eq!(tree.editable_composing_rect(root, preedit), None);
    tree.layout(constraints).unwrap();
    assert_eq!(
        tree.editable_composing_rect(root, preedit),
        Some(Rect::from_origin_size(
            Offset::new(13., 5.),
            Size::new(30., 10.)
        )),
    );
    assert_eq!(controller.text(), "1234");
    controller.clear_preedit();
    tree.layout(constraints).unwrap();
    assert_eq!(
        tree.editable_composing_rect(root, TextRange::new(1, 3))
            .unwrap()
            .size
            .width,
        20.,
    );
}

#[test]
fn composing_bounds_keep_local_scroll_offsets_and_zero_width_line_break_boxes() {
    let Some(style) = ahem() else { return };
    let controller = TextEditingController::with_text("AAA\nB\nCC\nDDDD");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .style(style.font_size(10.))
                .max_lines(None)
                .paragraph_layout(true)
                .into(),
        )
        .unwrap();
    tree.layout(Constraints::tight(Size::new(120., 20.)))
        .unwrap();
    assert!(
        tree.text_field_scroll_to_document_boundary(root, false)
            || tree.text_field_scroll_offset(root).unwrap().y == 0.
    );
    let range = TextRange::new(0, 3);
    let at_start = tree.editable_composing_rect(root, range).unwrap();
    assert_eq!(
        at_start,
        Rect::from_origin_size(Offset::ZERO, Size::new(30., 10.))
    );
    let newline = TextRange::new(3, 4);
    assert_eq!(
        tree.editable_composing_rect(root, newline),
        Some(Rect::from_origin_size(
            Offset::new(30., 0.),
            Size::new(0., 10.)
        ))
    );
    assert!(tree.text_field_scroll_to_document_boundary(root, true));
    assert_eq!(tree.text_field_scroll_offset(root).unwrap().y, 20.);
    let after = tree.editable_composing_rect(root, range).unwrap();
    assert_eq!(
        tree.editable_composing_rect(root, newline),
        Some(Rect::from_origin_size(
            Offset::new(30., -20.),
            Size::new(0., 10.)
        ))
    );
    assert_eq!(
        after,
        Rect::from_origin_size(Offset::new(0., -20.), at_start.size)
    );
    assert_eq!(
        tree.editable_composing_rect(root, TextRange::new(6, 8))
            .unwrap()
            .origin
            .y,
        0.
    );
}

#[test]
fn obscured_composing_bounds_use_the_mask_not_the_original_scalar_count() {
    let Some(style) = ahem() else { return };
    let controller = TextEditingController::with_text("a😀b");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller)
                .text_direction(TextDirection::Ltr)
                .style(style.font_size(10.))
                .obscure_text(true)
                .obscuring_character('X')
                .into(),
        )
        .unwrap();
    tree.layout(Constraints::tight(Size::new(100., 10.)))
        .unwrap();
    assert_eq!(
        tree.editable_composing_rect(root, TextRange::new(1, 5)),
        Some(Rect::from_origin_size(
            Offset::new(10., 0.),
            Size::new(20., 10.)
        )),
    );
    assert_eq!(
        tree.editable_composing_rect(root, TextRange::new(1, 2)),
        None
    );
}

#[test]
fn a_partial_composing_range_excludes_a_combined_glyph() {
    const FAMILY: &str = "Audit Composing Inter";
    register_application_font_as(
        FAMILY,
        include_bytes!("../../../harness/flutter/fonts/Inter-Regular.ttf").to_vec(),
    );
    let controller = TextEditingController::with_text("a\u{0301}");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller)
                .text_direction(TextDirection::Ltr)
                .style(TextStyle::new().font_family(FAMILY).font_size(20.))
                .into(),
        )
        .unwrap();
    tree.layout(Constraints::tight(Size::new(120., 30.)))
        .unwrap();
    assert_eq!(
        tree.editable_composing_rect(root, TextRange::new(0, 1)),
        None
    );
    assert_eq!(
        tree.editable_composing_rect(root, TextRange::new(1, 3)),
        None
    );
    assert!(
        tree.editable_composing_rect(root, TextRange::new(0, 3))
            .unwrap()
            .size
            .width
            > 0.
    );
}
