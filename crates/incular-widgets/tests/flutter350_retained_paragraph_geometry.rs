use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_text::{RichText, TextEngine, TextRange, TextSelection, TextSpan, TextStyle};
use incular_widgets::{
    Align, Directionality, GlobalKey, SelectionAreaController, Widget, internal::WidgetTree,
};

const SOURCE: &str =
    "I polished up that handle so carefullee\nThat now I am the Ruler of the Queen's Navee!";

const FAMILY: &str = "Pinned retained paragraph FlutterTest";

fn flutter_test_style() -> Option<(TextStyle, TextEngine)> {
    let font = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH")?;
    // flutter_tester's FontCollection::RegisterTestFonts disables system
    // fallback. ASCII apostrophe is absent from the embedded face and must
    // not pick a larger host font that shifts the whole line's baseline.
    let mut engine = TextEngine::with_system_fonts(false);
    assert!(engine.register_font_bytes_as(FAMILY, std::fs::read(font).unwrap()));
    Some((TextStyle::new().font_family(FAMILY), engine))
}

fn mount(
    span: TextSpan,
    engine: TextEngine,
    width: f32,
    selectable: bool,
) -> (WidgetTree, GlobalKey) {
    let key = GlobalKey::new();
    let paragraph = Widget::from(RichText::new(span)).with_key(key);
    let paragraph = if selectable {
        Widget::selection_area(SelectionAreaController::new(), paragraph)
    } else {
        paragraph
    };
    let root = Directionality::new(
        TextDirection::Ltr,
        Align::new(Alignment::TOP_LEFT, paragraph),
    );
    let mut tree = WidgetTree::with_text_engine(engine);
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(width, 600.)))
        .unwrap();
    (tree, key)
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_1,
        "{actual} != {expected}"
    );
}

fn edges(rect: Rect) -> [f32; 4] {
    [
        rect.origin.x,
        rect.origin.y,
        rect.origin.x + rect.size.width,
        rect.origin.y + rect.size.height,
    ]
}

#[test]
fn laid_out_selection_boxes_have_the_expected_tight_edges_across_lines() {
    // rendering/paragraph_test.dart:146
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let (tree, key) = mount(
        TextSpan::new(SOURCE).style(style.font_size(10.)),
        engine,
        800.,
        false,
    );
    let element = tree.element_with_key(&key.into()).unwrap();
    let one = tree
        .paragraph_selection_boxes(element, TextRange::new(5, 25))
        .unwrap();
    assert_eq!(one.len(), 1);
    let two = tree
        .paragraph_selection_boxes(element, TextRange::new(25, 50))
        .unwrap();
    assert_eq!(two.len(), 2);
    close(two[0].rect.origin.x, 250.);
    close(two[0].rect.origin.y, 0.);
    close(two[1].rect.origin.y, 10.);
    close(edges(two[1].rect)[2], 100.);
}

#[test]
fn each_font_run_keeps_its_own_baseline_aligned_tight_box() {
    // rendering/paragraph_test.dart:165
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::new("First ")
        .style(style.font_size(10.))
        .child(TextSpan::new("smallsecond ").style(TextStyle::new().font_size(5.)))
        .child(TextSpan::new("third fourth fifth"));
    let (tree, key) = mount(span, engine, 140., false);
    let element = tree.element_with_key(&key.into()).unwrap();
    let boxes = tree
        .paragraph_selection_boxes(element, TextRange::new(0, 36))
        .unwrap();
    assert_eq!(boxes.len(), 4, "boxes: {boxes:?}");
    for (actual, expected) in boxes.into_iter().zip([
        [0., 0., 60., 10.],
        [60., 3.75, 120., 8.75],
        [0., 10., 130., 20.],
        [0., 20., 50., 30.],
    ]) {
        assert_eq!(actual.direction, TextDirection::Ltr);
        for (got, want) in edges(actual.rect).into_iter().zip(expected) {
            close(got, want);
        }
    }
}

#[test]
fn laid_out_caret_hit_follows_horizontal_progress_and_line_breaks() {
    // rendering/paragraph_test.dart:129
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let (tree, key) = mount(
        TextSpan::new(SOURCE).style(style.font_size(14.)),
        engine,
        800.,
        false,
    );
    let element = tree.element_with_key(&key.into()).unwrap();
    let at = |x, y| {
        tree.selectable_text_position_at(element, Offset::new(x, y))
            .unwrap()
    };
    assert!(at(20., 5.) > 0);
    assert!(at(40., 5.) > at(20., 5.));
    assert!(at(5., 20.) > at(40., 5.));
}

#[test]
fn selecting_by_pointer_uses_the_tall_lines_real_vertical_bounds() {
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::new("a\n")
        .style(style.font_size(10.).height(Some(1.)))
        .child(TextSpan::new("bright\n").style(TextStyle::new().font_size(30.)))
        .child(TextSpan::new("last"));
    let (mut tree, key) = mount(span, engine, 800., true);
    let element = tree.element_with_key(&key.into()).unwrap();
    let bounds = tree.element_bounds(element).unwrap();
    let point = bounds.origin + Offset::new(25., 25.);
    assert_eq!(tree.selectable_text_position_at(element, point), Some(3));
    assert!(tree.selectable_text_set_selection(element, point, false));
    assert_eq!(
        tree.selectable_text_editing_value(element).unwrap().1,
        TextSelection::collapsed(3)
    );
}

#[test]
fn caret_origin_progresses_across_glyphs_and_through_hard_breaks() {
    // rendering/paragraph_test.dart:99
    use incular_text::TextAffinity::Downstream;

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let (mut tree, key) = mount(
        TextSpan::new(SOURCE).style(style.font_size(14.)),
        engine,
        800.,
        false,
    );
    let element = tree.element_with_key(&key.into()).unwrap();
    let mut at = |byte| {
        tree.paragraph_caret_metrics(element, byte, Downstream, 2.)
            .unwrap()
            .0
    };
    let first = at(5);
    assert!(first.x > 0.);
    assert!(at(25).x > first.x);
    assert!(at(50).y > first.y);
}

#[test]
fn caret_height_uses_the_shaped_fonts_natural_metrics() {
    // rendering/paragraph_test.dart:118
    use incular_text::TextAffinity::Downstream;

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let (mut tree, key) = mount(
        TextSpan::new(SOURCE).style(style.font_size(10.)),
        engine,
        800.,
        false,
    );
    let element = tree.element_with_key(&key.into()).unwrap();
    let (_, height) = tree
        .paragraph_caret_metrics(element, 5, Downstream, 2.)
        .unwrap();
    assert_eq!(height, 10.);
}

#[test]
fn laid_out_word_boundaries_preserve_spaces_and_embedded_apostrophes() {
    // rendering/paragraph_test.dart:252
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let (tree, key) = mount(
        TextSpan::new(SOURCE).style(style.font_size(14.)),
        engine,
        800.,
        false,
    );
    let element = tree.element_with_key(&key.into()).unwrap();
    for (byte, word) in [(5, "polished"), (50, " "), (75, "Queen's")] {
        let bounds = tree.paragraph_word_boundary(element, byte).unwrap();
        assert_eq!(&SOURCE[bounds.as_range()], word);
    }
}
