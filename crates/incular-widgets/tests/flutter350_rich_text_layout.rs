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
