//! `WidgetSpan` children of a rich paragraph (Flutter `RichText` /
//! `RenderParagraph`) and `Text.strutStyle`.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_text::{FontFamily, StrutStyle, TextSpan, TextStyle, WidgetSpanAlignment};
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::{Directionality, RichParagraph, RichText, Text, Widget, WidgetSpan};

fn inter() -> TextStyle {
    // Application fonts are registered per thread.
    thread_local! {
        static FONT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if !FONT.replace(true) {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../harness/flutter/fonts/Inter-Regular.ttf"
        ))
        .expect("read harness/flutter/fonts/Inter-Regular.ttf");
        incular_text::register_application_font(bytes);
    }
    TextStyle::new()
        .family(FontFamily::named("Inter"))
        .size(14.)
        .color(Color::BLACK)
}

fn mount(widget: Widget) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, widget).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(220., 400.)))
        .expect("layout");
    (tree, root)
}

/// The paragraph element and its descendants, depth first.
fn descendants(tree: &WidgetTree, id: ElementId, out: &mut Vec<ElementId>) {
    out.push(id);
    for &child in tree.children(id).unwrap_or(&[]) {
        descendants(tree, child, out);
    }
}

fn paragraph_and_box(tree: &WidgetTree, root: ElementId) -> (ElementId, ElementId) {
    let mut all = Vec::new();
    descendants(tree, root, &mut all);
    let paragraph = *all
        .iter()
        .find(|&&id| {
            tree.render_id(id)
                .and_then(|render| tree.render_object_kind(render))
                .is_some_and(|kind| matches!(kind, RenderKind::Text { .. }))
        })
        .expect("paragraph");
    let child = *all
        .iter()
        .rev()
        .find(|&&id| {
            tree.render_id(id)
                .and_then(|render| tree.render_object_kind(render))
                .is_some_and(|kind| matches!(kind, RenderKind::Box { .. }))
        })
        .expect("inline child");
    (paragraph, child)
}

fn widget_span(alignment: WidgetSpanAlignment) -> RichParagraph {
    let span = TextSpan::new("before ")
        .style(inter())
        .child(WidgetSpan::new(7).alignment(alignment))
        .child(TextSpan::new(" after"));
    RichParagraph::new(RichText::new(span)).child(
        7,
        Widget::box_(Size::new(40., 40.), Color::rgba(0, 0, 255, 255)),
    )
}

#[test]
fn a_widget_span_child_is_laid_out_placed_and_hit_in_its_box() {
    let (tree, root) = mount(widget_span(WidgetSpanAlignment::Middle).into());
    let (paragraph, child) = paragraph_and_box(&tree, root);
    let bounds = tree.element_bounds(paragraph).expect("paragraph bounds");
    // text__widget_span_layout: Flutter's paragraph is 220x40 around the
    // middle-aligned 40x40 child (the Align-free probe fills the width).
    assert_eq!(bounds.size.height, 40.);
    let child_bounds = tree.element_bounds(child).expect("child bounds");
    assert_eq!(child_bounds.size, Size::new(40., 40.));
    assert!(child_bounds.origin.x > bounds.origin.x + 30.);
    assert!((child_bounds.origin.y - bounds.origin.y).abs() < 0.01);

    let layout = tree
        .text_layout(tree.render_id(paragraph).expect("render"))
        .expect("text layout");
    let placed = layout.placeholders[0];
    assert!((placed.rect.origin.x - (child_bounds.origin.x - bounds.origin.x)).abs() < 0.01);

    // The child is hit where it is painted.
    let center = Offset::new(child_bounds.origin.x + 20., child_bounds.origin.y + 20.);
    let hit = tree.hit_test(center).expect("hit");
    assert_ne!(Some(hit), tree.render_id(paragraph));
}

#[test]
fn a_childless_widget_span_reserves_its_declared_size() {
    let span = TextSpan::new("a ")
        .style(inter())
        .child(WidgetSpan::sized(1, Size::new(10., 30.)).alignment(WidgetSpanAlignment::TextTop));
    let (tree, root) = mount(RichText::new(span).into());
    let mut all = Vec::new();
    descendants(&tree, root, &mut all);
    let paragraph = all
        .into_iter()
        .find(|&id| {
            tree.render_id(id)
                .and_then(|r| tree.text_layout(r))
                .is_some()
        })
        .expect("paragraph");
    let layout = tree
        .text_layout(tree.render_id(paragraph).expect("render"))
        .expect("layout");
    assert_eq!(layout.placeholders[0].rect.size, Size::new(10., 30.));
    // A top-aligned box starts at the font's ascent line and hangs below.
    assert!(layout.metrics.size.height >= 30.);
}

#[test]
fn a_forced_strut_fixes_the_text_line_height() {
    let strut = StrutStyle::new()
        .font_family("Inter")
        .font_size(30.)
        .force_strut_height(true);
    let (tree, root) = mount(
        Text::new("one\ntwo")
            .style(inter())
            .strut_style(strut)
            .into(),
    );
    let mut all = Vec::new();
    descendants(&tree, root, &mut all);
    let paragraph = all
        .into_iter()
        .find(|&id| {
            tree.render_id(id)
                .and_then(|r| tree.text_layout(r))
                .is_some()
        })
        .expect("paragraph");
    let height = tree.element_bounds(paragraph).expect("bounds").size.height;
    // Inter's ascent + descent is 1.2109375 em: 36.33 px at 30 px, snapped
    // to 36 per line.
    assert_eq!(height, 72.);
}
