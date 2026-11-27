use std::sync::Arc;

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_text::{RichText, TextSpan, TextStyle};
use incular_widgets::{Align, Directionality, GlobalKey, Widget, internal::WidgetTree};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(800., 100.))
}

fn paragraph(key: GlobalKey, span: TextSpan) -> Widget {
    Widget::from(RichText::new(span)).with_key(key)
}

fn scene(key: GlobalKey, span: TextSpan) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        Align::new(incular_config::Alignment::TOP_LEFT, paragraph(key, span)),
    )
    .into()
}

fn colors(tree: &mut WidgetTree) -> Vec<(u32, Color)> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|cmd| match cmd {
            PaintCommand::GlyphRun { run, color } => Some((run, color)),
            _ => None,
        })
        .flat_map(|(run, color)| run.glyphs.iter().map(|glyph| (glyph.cluster, *color)))
        .collect()
}

#[test]
fn content_updates_layout_but_recoloring_repaints_existing_glyphs() {
    // Exact rendering/paragraph_test.dart:507 content/color transitions.
    let key = GlobalKey::new();
    let inner = |text, color| TextSpan::new(text).style(TextStyle::new().color(color));
    let mut tree = WidgetTree::new();
    tree.mount(scene(key, inner("Hello", Color::BLACK)))
        .unwrap();
    tree.layout(constraints()).unwrap();
    assert!(!colors(&mut tree).is_empty());
    let before = tree.diagnostics().layouts;
    let element = tree.element_with_key(&key.into()).unwrap();

    tree.update(element, paragraph(key, inner("Hello World", Color::BLACK)))
        .unwrap();
    tree.layout(constraints()).unwrap();
    assert!(
        tree.diagnostics().layouts > before,
        "text change needs layout"
    );
    let black = colors(&mut tree);
    assert!(black.iter().all(|(_, color)| *color == Color::BLACK));
    let render = tree.render_id(element).unwrap();
    let retained_clusters = tree.text_layout(render).unwrap().lines[0].clusters.clone();
    let before = tree.diagnostics().layouts;

    tree.update(element, paragraph(key, inner("Hello World", Color::WHITE)))
        .unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(tree.diagnostics().layouts, before, "color-only update");
    assert!(Arc::ptr_eq(
        &retained_clusters,
        &tree.text_layout(render).unwrap().lines[0].clusters
    ));
    let white = colors(&mut tree);
    assert_eq!(black.len(), white.len());
    assert!(white.iter().all(|(_, color)| *color == Color::WHITE));
}

#[test]
fn per_span_recoloring_updates_actual_mixed_glyph_output_without_layout() {
    const RED: Color = Color::rgba(255, 0, 0, 255);
    const BLUE: Color = Color::rgba(0, 0, 255, 255);
    let key = GlobalKey::new();
    let inner = |first, second| {
        TextSpan::new("x")
            .style(TextStyle::new().color(first))
            .child(TextSpan::new("y").style(TextStyle::new().color(second)))
    };
    let mut tree = WidgetTree::new();
    tree.mount(scene(key, inner(RED, BLUE))).unwrap();
    tree.layout(constraints()).unwrap();
    assert_eq!(colors(&mut tree), [(0, RED), (1, BLUE)]);
    let before = tree.diagnostics().layouts;
    let element = tree.element_with_key(&key.into()).unwrap();

    // Change both mixed colors, collapse their map to a single color, and
    // then restore a mixed map without losing the positions of either glyph.
    for (first, second) in [(BLUE, RED), (RED, RED), (RED, BLUE)] {
        tree.update(element, paragraph(key, inner(first, second)))
            .unwrap();
        tree.layout(constraints()).unwrap();
        assert_eq!(tree.diagnostics().layouts, before);
        assert_eq!(colors(&mut tree), [(0, first), (1, second)]);
    }
}
