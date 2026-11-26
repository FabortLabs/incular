//! Flutter paint-versus-color precedence through retained glyph commands.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Lerp, Size};
use incular_rendering::PaintCommand;
use incular_text::TextStyle;
use incular_widgets::{DefaultTextStyle, Text, Widget, internal::WidgetTree};

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn glyph_color(child: impl Into<Widget>) -> Color {
    let mut tree = WidgetTree::new();
    tree.mount(child.into()).unwrap();
    tree.layout(Constraints::loose(Size::new(600., 80.)))
        .unwrap();
    tree.paint()
        .commands()
        .iter()
        .find_map(|cmd| match cmd {
            PaintCommand::GlyphRun { color, .. } => Some(*color),
            _ => None,
        })
        .expect("painted glyph color")
}

fn text(style: TextStyle) -> Text {
    Text::new("Foreground")
        .text_direction(TextDirection::Ltr)
        .style(style)
}

// painting/text_style_test.dart:506.
#[test]
fn foreground_beats_simple_color_when_merging_and_applying_styles() {
    let red = TextStyle::new().color(RED);
    let blue = TextStyle::new().color(BLUE);
    let red_paint = TextStyle::new().foreground(RED);
    let blue_paint = TextStyle::new().foreground(BLUE);
    for (style, outward) in [
        (red.merge(&blue), BLUE),
        (red_paint.merge(&blue_paint), BLUE),
        (red_paint.merge(&blue), RED),
        (blue.merge(&red_paint), RED),
        (
            red_paint.apply(Some(BLUE), None, None, None, None, None),
            RED,
        ),
        (red.apply(Some(BLUE), None, None, None, None, None), BLUE),
    ] {
        assert_eq!(glyph_color(text(style)), outward);
    }
}

#[test]
fn a_foreground_tween_selects_the_endpoint_at_half_instead_of_blending() {
    let red = TextStyle::new().color(RED);
    let blue = TextStyle::new().color(BLUE);
    let red_paint = TextStyle::new().foreground(RED);
    let blue_paint = TextStyle::new().foreground(BLUE);
    assert_eq!(
        glyph_color(text(red.lerp(&blue, 0.25))),
        RED.lerp(&BLUE, 0.25)
    );
    for start in [&red, &red_paint] {
        for (t, expected) in [(0.25, RED), (0.5, BLUE), (0.75, BLUE)] {
            let candidate = start.lerp(&blue_paint, t);
            assert_eq!(candidate.foreground, Some(expected));
            assert_eq!(glyph_color(text(candidate)), expected);
        }
    }
}

#[test]
fn an_applied_color_overrides_inherited_plain_color() {
    let overriding = TextStyle::new().apply(Some(BLUE), None, None, None, None, None);
    let scene = DefaultTextStyle::new(TextStyle::new().color(RED), text(overriding));
    assert_eq!(glyph_color(scene), BLUE);
}

#[test]
fn rich_spans_paint_their_own_explicit_foregrounds() {
    use incular_text::TextSpan;
    use incular_widgets::{Directionality, RichText};

    let span = TextSpan::new("").children([
        TextSpan::new("red").style(TextStyle::new().foreground(RED)),
        TextSpan::new("blue").style(TextStyle::new().foreground(BLUE)),
    ]);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, RichText::new(span)).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(600., 80.)))
        .unwrap();
    let colors: Vec<_> = tree
        .paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::GlyphRun { color, .. } => Some(*color),
            _ => None,
        })
        .collect();
    assert_eq!(colors, [RED, BLUE]);
}

#[test]
fn foreground_only_update_repaints_without_reshaping_or_replacing_text() {
    let mut tree = WidgetTree::new();
    let bounds = Constraints::loose(Size::new(600., 80.));
    let root = tree
        .mount(text(TextStyle::new().foreground(RED)).into())
        .unwrap();
    tree.layout(bounds).unwrap();
    let _ = tree.paint();
    let measured = tree.root_layout_size();
    let render = tree.render_id(root);
    let layouts = tree.diagnostics().layouts;
    tree.update(root, text(TextStyle::new().foreground(BLUE)).into())
        .unwrap();
    tree.layout(bounds).unwrap();
    let display = tree.paint();
    assert_eq!(tree.root_layout_size(), measured);
    assert_eq!(tree.render_id(root), render);
    assert_eq!(tree.diagnostics().layouts, layouts);
    assert!(display.commands().iter().any(|command| matches!(
        command, PaintCommand::GlyphRun { color, .. } if *color == BLUE
    )));
}
