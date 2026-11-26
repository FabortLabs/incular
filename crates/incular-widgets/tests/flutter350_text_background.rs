//! Flutter text background geometry, paint precedence, and paint-only updates.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Lerp, Rect, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_text::{TextAlign, TextSpan, TextStyle};
use incular_widgets::{Directionality, RichText, Text, Widget, internal::WidgetTree};

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn paragraph(style: TextStyle, contents: &str) -> Text {
    Text::new(contents)
        .text_direction(TextDirection::Ltr)
        .align(TextAlign::Right)
        .style(style.color(Color::WHITE).size(23.))
}

fn rects(list: &DisplayList) -> Vec<(Rect, Color)> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { rect, color } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

#[test]
fn plain_text_background_covers_each_painted_line_and_stops_at_its_advance() {
    let bounds = Constraints::tight(Size::new(180., 100.));
    let style = TextStyle::new().background_color(RED);
    let mut tree = WidgetTree::new();
    let element = tree.mount(paragraph(style, "ABC\nA").into()).unwrap();
    tree.layout(bounds).unwrap();
    let list = tree.paint();
    let backgrounds = rects(&list);
    assert_eq!(backgrounds.len(), 2);
    let layout = tree.text_layout(tree.render_id(element).unwrap()).unwrap();
    for (&(rect, color), line) in backgrounds.iter().zip(layout.lines.iter()) {
        assert_eq!(color, RED);
        assert_eq!(rect.origin.x, line.offset);
        assert_eq!(rect.origin.x + rect.size.width, line.offset + line.width);
        assert!(rect.origin.y < line.baseline);
        assert!(rect.origin.y + rect.size.height > line.baseline);
    }
    assert!(backgrounds[1].0.origin.x > backgrounds[0].0.origin.x);
    assert!(backgrounds[1].0.origin.y > backgrounds[0].0.origin.y);
    let first_glyph = list
        .commands()
        .iter()
        .position(|cmd| matches!(cmd, PaintCommand::GlyphRun { .. }))
        .unwrap();
    assert!(
        list.commands()[..first_glyph]
            .iter()
            .any(|cmd| matches!(cmd, PaintCommand::Rect { color, .. } if *color == RED))
    );
}

#[test]
fn background_paint_overrides_plain_color_and_tweens_by_endpoint() {
    let red = TextStyle::new().background_color(RED);
    let blue = TextStyle::new().background_color(BLUE);
    let red_paint = TextStyle::new().background(RED);
    let blue_paint = TextStyle::new().background(BLUE);
    assert_eq!(red.merge(&blue).effective_background_color(), Some(BLUE));
    for merged in [
        red_paint.merge(&blue),
        blue.merge(&red_paint),
        red_paint.apply(None, Some(BLUE), None, None, None, None),
    ] {
        assert_eq!(merged.background, Some(RED));
        assert_eq!(merged.background_color, None);
    }
    assert_eq!(red_paint.merge(&blue_paint).background, Some(BLUE));
    assert_eq!(
        red.apply(None, Some(BLUE), None, None, None, None)
            .background_color,
        Some(BLUE)
    );
    assert_eq!(
        red.lerp(&blue, 0.25).background_color,
        Some(RED.lerp(&BLUE, 0.25))
    );
    assert_eq!(
        TextStyle::new().lerp(&red, 0.25).background_color,
        Some(Color::rgba(255, 0, 0, 64))
    );
    for style in [&red, &red_paint] {
        for (t, selected) in [(0.25, RED), (0.5, BLUE), (0.75, BLUE)] {
            let mixed = style.lerp(&blue_paint, t);
            assert_eq!(mixed.background, Some(selected));
            assert!(mixed.background_color.is_none());
        }
    }
}

#[test]
fn adding_a_background_then_recoloring_keeps_text_identity_and_reuses_geometry() {
    let mut tree = WidgetTree::new();
    let bounds = Constraints::loose(Size::new(180., 80.));
    let original = paragraph(TextStyle::new(), "color");
    let root = tree.mount(original.into()).unwrap();
    tree.layout(bounds).unwrap();
    let stable = tree.render_id(root);
    assert!(rects(&tree.paint()).is_empty());
    tree.update(
        root,
        paragraph(TextStyle::new().background(RED), "color").into(),
    )
    .unwrap();
    tree.layout(bounds).unwrap();
    let first = rects(&tree.paint());
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].1, RED);
    let layouts = tree.diagnostics().layouts;
    tree.update(
        root,
        paragraph(TextStyle::new().background_color(BLUE), "color").into(),
    )
    .unwrap();
    tree.layout(bounds).unwrap();
    assert_eq!(rects(&tree.paint()), [(first[0].0, BLUE)]);
    assert_eq!(tree.render_id(root), stable);
    assert_eq!(tree.diagnostics().layouts, layouts);

    tree.update(
        root,
        paragraph(TextStyle::new().background(Color::TRANSPARENT), "color").into(),
    )
    .unwrap();
    tree.layout(bounds).unwrap();
    assert!(rects(&tree.paint()).is_empty());
}

#[test]
fn rich_spans_only_paint_their_own_background_ranges() {
    let span = TextSpan::new("").children([
        TextSpan::new("A").style(TextStyle::new().background_color(RED)),
        TextSpan::new("B"),
        TextSpan::new("C").style(TextStyle::new().background(BLUE)),
    ]);
    let widget: Widget = Directionality::new(TextDirection::Ltr, RichText::new(span)).into();
    let mut tree = WidgetTree::new();
    tree.mount(widget).unwrap();
    tree.layout(Constraints::loose(Size::new(180., 80.)))
        .unwrap();
    let positions = rects(&tree.paint());
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0].1, RED);
    assert_eq!(positions[1].1, BLUE);
    assert!(positions[0].0.origin.x + positions[0].0.size.width < positions[1].0.origin.x);
}

#[test]
fn explicit_text_height_is_included_in_the_background_not_just_the_glyphs() {
    let bounds = Constraints::loose(Size::new(180., 100.));
    let style = TextStyle::new()
        .background_color(RED)
        .line_height_multiplier(2.);
    let mut tree = WidgetTree::new();
    tree.mount(paragraph(style, "A").into()).unwrap();
    tree.layout(bounds).unwrap();
    let [(rect, color)] = rects(&tree.paint())
        .try_into()
        .expect("one typographic background");
    assert_eq!(color, RED);
    assert!((rect.size.height - 46.).abs() < 1e-5);
}
