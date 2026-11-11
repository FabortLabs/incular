//! Text shadows paint, and aligned text inside a shrink-wrapped box stays
//! inside that box.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_text::{TextAlign, TextOverflow, TextStyle};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Align, Shadow, Text, Widget};

/// Every glyph run with its color and the world-space position of its first
/// glyph, in paint order.
fn glyph_runs(list: &DisplayList) -> Vec<(Color, Offset)> {
    let mut transforms = vec![Offset::ZERO];
    let mut runs = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                transforms.push(*transforms.last().unwrap() + transform.translation_offset());
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::GlyphRun { run, color } => {
                let first = run
                    .glyphs
                    .first()
                    .map_or(Offset::ZERO, |glyph| glyph.offset);
                runs.push((*color, run.origin + first + *transforms.last().unwrap()));
            }
            _ => {}
        }
    }
    runs
}

fn paint(widget: Widget, size: Size) -> (WidgetTree, DisplayList) {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::tight(size)).expect("layout");
    let list = tree.paint();
    (tree, list)
}

fn shadowed(shadows: Vec<Shadow>) -> Widget {
    Text::new("Shadow")
        .text_direction(TextDirection::Ltr)
        .style(
            TextStyle::default()
                .size(20.)
                .color(Color::WHITE)
                .shadows(shadows),
        )
        .into()
}

#[test]
fn sharp_text_shadows_paint_offset_copies_beneath_the_text() {
    let red = Color::rgba(255, 0, 0, 255);
    let (_, list) = paint(
        Align::new(
            incular_config::Alignment::TOP_LEFT,
            shadowed(vec![Shadow::new(red, Offset::new(3., 4.), 0.)]),
        )
        .into(),
        Size::new(300., 100.),
    );
    let runs = glyph_runs(&list);
    let shadow = runs
        .iter()
        .position(|(color, _)| *color == red)
        .expect("shadow glyphs are painted");
    let text = runs
        .iter()
        .position(|(color, _)| *color == Color::WHITE)
        .expect("text glyphs are painted");
    assert!(shadow < text, "the shadow paints beneath the text");
    let delta = runs[shadow].1 - runs[text].1;
    assert!(
        (delta.x - 3.).abs() < 0.01 && (delta.y - 4.).abs() < 0.01,
        "shadow offset {delta:?}"
    );
}

#[test]
fn blurred_text_shadows_use_a_drop_shadow_layer() {
    let shadow_color = Color::rgba(0, 0, 0, 200);
    let (_, list) = paint(
        shadowed(vec![Shadow::new(shadow_color, Offset::new(2., 2.), 6.)]),
        Size::new(300., 100.),
    );
    assert!(
        list.commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::PushDropShadow { .. })),
        "a blurred text shadow must be composited through a drop shadow"
    );
    // The blurred shadow is the layer's job; it is not also drawn sharp.
    assert!(
        glyph_runs(&list)
            .iter()
            .all(|(color, _)| *color != shadow_color)
    );
}

#[test]
fn centered_text_in_a_shrink_wrapped_box_stays_inside_it() {
    for align in [TextAlign::Center, TextAlign::End, TextAlign::Right] {
        let (tree, list) = paint(
            Align::new(
                incular_config::Alignment::TOP_LEFT,
                Text::new("Short\nA considerably longer caption line")
                    .text_direction(TextDirection::Ltr)
                    .align(align)
                    .overflow(TextOverflow::Clip)
                    .style(TextStyle::default().size(16.).color(Color::WHITE)),
            )
            .into(),
            Size::new(800., 200.),
        );
        let root = tree.root().expect("root");
        let text = tree.children(root).expect("align child")[0];
        let bounds = tree.element_bounds(text).expect("text bounds");
        assert!(
            bounds.size.width < 600.,
            "{align:?}: the text shrink-wraps ({bounds:?})"
        );
        for (_, origin) in glyph_runs(&list) {
            assert!(
                origin.x >= bounds.origin.x - 0.5 && origin.x < bounds.origin.x + bounds.size.width,
                "{align:?}: a line starts at x {} outside its {bounds:?} box",
                origin.x
            );
        }
    }
}
