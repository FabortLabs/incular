//! Mounted rich paragraphs against the Flutter 3.50.0 widget scenarios.

use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_text::{TextSpan, TextStyle, WidgetSpanAlignment};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Align, ColoredBox, Directionality, GlobalKey, IntrinsicHeight, RichParagraph, RichText, Row,
    SizedBox, Widget, WidgetSpan,
};

fn size(tree: &WidgetTree, key: GlobalKey) -> Size {
    let element = tree.element_with_key(&key.into()).unwrap();
    tree.element_bounds(element).unwrap().size
}

fn ahem() -> TextStyle {
    const FAMILY: &str = "Pinned RichText Ahem";
    let font = std::env::var("INCULAR_FLUTTER_AHEM_PATH")
        .expect("point INCULAR_FLUTTER_AHEM_PATH to the pinned Flutter Ahem.ttf");
    incular_text::register_application_font_as(FAMILY, std::fs::read(font).unwrap());
    TextStyle::new()
        .font_family(FAMILY)
        .font_size(16.)
        .height(Some(1.))
}

#[test]
fn an_inline_row_between_text_runs_yields_three_sixteen_pixel_intrinsic_lines() {
    let Some(_) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let style = ahem();
    compare_inline_row(&style, 16.);
    for height in [4., 10., 14., 18., 32.] {
        compare_inline_row(&style, height);
    }
}

fn compare_inline_row(style: &TextStyle, height: f32) {
    let intrinsic = GlobalKey::new();
    let inline = GlobalKey::new();
    let paragraph = GlobalKey::new();
    let span = TextSpan::empty()
        .child(TextSpan::new("Start\n").style(style.clone()))
        .child(WidgetSpan::new(1))
        .child(TextSpan::new("End").style(style.clone()));
    let content = RichParagraph::new(RichText::new(span)).child(
        1,
        Widget::from(Row::new([SizedBox::new().width(16.).height(height)])).with_key(inline),
    );
    let wrap = Widget::from(IntrinsicHeight::new(
        Widget::from(content).with_key(paragraph),
    ))
    .with_key(intrinsic);
    let root = Directionality::new(
        TextDirection::Ltr,
        Align::new(
            Alignment::CENTER,
            ColoredBox::new(Color::rgba(0, 255, 0, 255), wrap),
        ),
    );
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(size(&tree, intrinsic).height, 32. + height);
    assert_eq!(size(&tree, paragraph).height, 32. + height);
    assert_eq!(size(&tree, inline).height, height);
    let element = tree.element_with_key(&paragraph.into()).unwrap();
    let layout = tree.text_layout(tree.render_id(element).unwrap()).unwrap();
    let heights: Vec<_> = layout.lines.iter().map(|line| line.height).collect();
    assert_eq!(heights, [16., height, 16.]);
    let [placeholder] = layout.placeholders.as_ref() else {
        panic!("expected one placed inline row");
    };
    assert_eq!(placeholder.line, 1);
    if height == 16. {
        assert_eq!(placeholder.rect.origin.y, 16.);
    }
    assert_eq!(placeholder.rect.size.height, height);
}

#[test]
fn a_rich_paragraph_lays_out_and_paints_under_a_zero_area_view() {
    let key = GlobalKey::new();
    let rich = Widget::from(RichText::new(TextSpan::new("text"))).with_key(key);
    let scene = Directionality::new(TextDirection::Ltr, Align::new(Alignment::CENTER, rich));
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    tree.update_compositor(std::time::Instant::now()).unwrap();
    assert_eq!(size(&tree, key), Size::ZERO);
}

#[test]
fn widget_span_defaults_to_bottom_alignment() {
    for span in [
        WidgetSpan::new(1),
        WidgetSpan::sized(1, Size::new(20., 16.)),
    ] {
        assert_eq!(span.alignment, WidgetSpanAlignment::TextBottom);
    }
    assert_eq!(
        WidgetSpanAlignment::default(),
        WidgetSpanAlignment::TextBottom
    );
}

#[test]
fn an_ellipsized_inline_widget_does_not_paint_hit_or_expose_semantics_at_the_origin() {
    use incular_core::Offset;
    use incular_rendering::PaintCommand;
    use incular_text::TextOverflow;
    use incular_widgets::Semantics;

    let Some(_) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let key = GlobalKey::new();
    let tint = Color::rgba(23, 187, 64, 255);
    let text = TextSpan::new("a long line to be truncated.")
        .style(ahem())
        .child(WidgetSpan::new(1));
    let rich = RichParagraph::new(
        RichText::new(text)
            .max_lines(Some(1))
            .overflow(TextOverflow::Ellipsis),
    )
    .child(
        1,
        Semantics::new(
            Widget::from(ColoredBox::new(
                tint,
                SizedBox::new().width(64.).height(16.),
            ))
            .with_key(key),
        )
        .label("truncated")
        .container(true),
    );
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, rich).into())
        .unwrap();
    let frame = |tree: &mut WidgetTree, width| {
        tree.layout(Constraints::loose(Size::new(width, 100.)))
            .unwrap();
        tree.update_semantics();
        tree.paint()
    };
    let painted_child = |list: &incular_rendering::DisplayList| {
        list.commands().iter().any(|command| {
            matches!(command, PaintCommand::Rect { color, .. }
                | PaintCommand::HardEdgeRect { color, .. } if *color == tint)
        })
    };
    let semantics_has_child = |tree: &WidgetTree| {
        tree.semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("truncated"))
    };

    assert!(painted_child(&frame(&mut tree, 800.)));
    let child = tree.element_with_key(&key.into()).unwrap();
    let render = tree.render_id(child).unwrap();
    let location = tree.element_bounds(child).unwrap().origin + Offset::new(1., 1.);
    assert_eq!(tree.hit_test(location), Some(render));
    assert!(semantics_has_child(&tree));

    let display_list = frame(&mut tree, 100.);
    let paragraph = tree.children(tree.root().unwrap()).unwrap()[0];
    let rich_render = tree.render_id(paragraph).unwrap();
    assert!(
        tree.text_layout(rich_render)
            .unwrap()
            .placeholders
            .is_empty()
    );
    assert_eq!(
        tree.element_with_key(&key.into()),
        Some(child),
        "keep retained state"
    );
    assert!(
        !painted_child(&display_list),
        "an elided inline widget must not leave a stale picture"
    );
    assert_ne!(tree.hit_test(Offset::new(1., 1.)), Some(render));
    assert!(!semantics_has_child(&tree));
    assert_eq!(
        tree.semantics()
            .iter()
            .filter_map(|(_, node)| node.label.as_deref())
            .collect::<Vec<_>>(),
        ["a long line to be truncated."]
    );

    assert!(painted_child(&frame(&mut tree, 800.)));
    assert_eq!(tree.hit_test(location), Some(render));
    assert!(semantics_has_child(&tree));
}

#[test]
fn unlimited_ellipsis_excludes_the_truncated_inline_render_child() {
    use incular_text::TextOverflow;
    use incular_widgets::Text;

    let Some(_) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let style = ahem().font_size(14.);
    let sentence = "truncated";
    let line = "a long line to be truncated.";
    let span = TextSpan::new(line)
        .style(style.clone())
        .child(WidgetSpan::new(1));
    let rich = RichParagraph::new(RichText::new(span).overflow(TextOverflow::Ellipsis))
        .child(1, Text::new(sentence).style(style));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, rich).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .unwrap();
    tree.update_semantics();

    let render = tree.render_id(tree.children(root).unwrap()[0]).unwrap();
    let layout = tree.text_layout(render).unwrap();
    assert!(layout.overflowed);
    assert_eq!(layout.lines.len(), 1);
    assert!(layout.placeholders.is_empty());
    let graph = tree.semantics();
    assert_eq!(graph.len(), 1);
    let root = graph.node(graph.roots()[0]).unwrap();
    assert_eq!(root.label.as_deref(), Some(line));
    assert!(root.children.is_empty());
}

// rendering/paragraph_test.dart:538. This reads the real retained layout,
// never the descriptor's independent convenience shaping API.
#[test]
fn inherited_span_fonts_are_scaled_once_in_the_mounted_paragraph() {
    use incular_text::TextScaler;

    let Some(_) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let span = TextSpan::new("a")
        .style(ahem().font_size(10.))
        .child(
            TextSpan::new("b")
                .style(TextStyle::new().font_size(20.))
                .child(TextSpan::new("c")),
        )
        .child(TextSpan::new("d"));
    for (factor, widths, expected) in [
        (1.3, [13., 26., 26., 13.], Size::new(78., 26.)),
        (1., [10., 20., 20., 10.], Size::new(60., 20.)),
    ] {
        let key = GlobalKey::new();
        let rich = RichText::new(span.clone()).text_scaler(TextScaler::linear(factor));
        let scene = Directionality::new(
            TextDirection::Ltr,
            Align::new(Alignment::CENTER, Widget::from(rich).with_key(key)),
        );
        let mut tree = WidgetTree::new();
        tree.mount(scene.into()).unwrap();
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        assert_eq!(size(&tree, key), expected);

        let element = tree.element_with_key(&key.into()).unwrap();
        let layout = tree.text_layout(tree.render_id(element).unwrap()).unwrap();
        assert_eq!(layout.lines.len(), 1);
        for (byte, expected_width) in widths.into_iter().enumerate() {
            let [(left, right)] = layout.lines[0].selection_spans(byte, byte + 1)[..] else {
                panic!("expected one selected box for byte {byte}");
            };
            assert_eq!(right - left, expected_width);
            let run = layout
                .font_runs
                .iter()
                .find(|run| run.range.contains(&byte))
                .unwrap();
            assert!((run.caret_height - expected_width).abs() < 0.000_1);
        }
    }
}

// rendering/paragraph_test.dart:488, :494 and :500. Height is unconstrained
// by the parent so the flag describes line truncation, not vertical clipping.
#[test]
fn retained_paragraph_reports_line_and_ellipsis_truncation_separately_from_wrapping() {
    use incular_text::TextOverflow;

    let Some(_) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let span =
        TextSpan::new("Here is a long text, maybe exceed maxlines").style(ahem().font_size(10.));
    for (max_lines, overflow, truncated) in [
        (None, TextOverflow::Clip, false),
        (Some(1), TextOverflow::Clip, true),
        (None, TextOverflow::Ellipsis, true),
    ] {
        let key = GlobalKey::new();
        let rich = RichText::new(span.clone())
            .max_lines(max_lines)
            .overflow(overflow);
        let scene = Directionality::new(
            TextDirection::Ltr,
            Align::new(Alignment::TOP_LEFT, Widget::from(rich).with_key(key)),
        );
        let mut tree = WidgetTree::new();
        tree.mount(scene.into()).unwrap();
        tree.layout(Constraints::tight(Size::new(100., 600.)))
            .unwrap();
        let element = tree.element_with_key(&key.into()).unwrap();
        let layout = tree.text_layout(tree.render_id(element).unwrap()).unwrap();
        assert_eq!(
            layout.overflowed, truncated,
            "max={max_lines:?}, overflow={overflow:?}"
        );
    }
}
