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

#[test]
fn changing_alignment_repositions_selection_and_glyphs_without_box_layout() {
    use incular_rendering::PaintCommand;
    use incular_text::TextAlign;
    use incular_widgets::SizedBox;

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let key = GlobalKey::new();
    let make = |align| {
        Widget::from(RichText::new(TextSpan::new("A").style(style.clone().font_size(10.))).align(align))
            .with_key(key)
    };
    let root = Directionality::new(
        TextDirection::Ltr,
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(100.).child(make(TextAlign::Left)),
        ),
    );
    let mut tree = WidgetTree::with_text_engine(engine);
    tree.mount(root.into()).unwrap();
    let limits = Constraints::tight(Size::new(800., 600.));
    tree.layout(limits).unwrap();
    let field = tree.element_with_key(&key.into()).unwrap();
    let render = tree.render_id(field).unwrap();
    let before = tree.diagnostics().layouts;

    for (align, x) in [(TextAlign::Left, 0.), (TextAlign::Right, 90.), (TextAlign::Center, 45.), (TextAlign::Left, 0.)] {
        tree.update(field, make(align)).unwrap();
        tree.layout(limits).unwrap();
        let painted = tree.paint();
        let bounds = tree.paragraph_selection_boxes(field, TextRange::new(0, 1)).unwrap();
        assert_eq!(bounds.len(), 1);
        assert_eq!(edges(bounds[0].rect), [x, 0., x + 10., 10.]);
        let glyph = painted.commands().iter().find_map(|command| match command {
            PaintCommand::GlyphRun { run, .. } => Some(run.origin.x + run.glyphs[0].offset.x),
            _ => None,
        });
        assert_eq!(glyph, Some(x));
        assert_eq!(tree.render_id(field), Some(render));
        assert_eq!(tree.diagnostics().layouts, before, "alignment {align:?}");
    }
}

#[test]
fn a_live_line_cap_changes_the_height_at_the_same_paragraph_width() {
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let source = "How do you write like you're running out of time? Write day and night like you're running out of time?";
    let span = TextSpan::new(source).style(style.font_size(10.));
    let (mut tree, key) = mount(span.clone(), engine, 100., false);
    let field = tree.element_with_key(&key.into()).unwrap();
    let render = tree.render_id(field).unwrap();
    let constraints = Constraints::tight(Size::new(100., 600.));
    for (max, height) in [(None, 130.), (Some(1), 10.), (Some(2), 20.), (Some(3), 30.), (None, 130.)] {
        tree.update(field, Widget::from(RichText::new(span.clone()).max_lines(max)).with_key(key))
            .unwrap();
        tree.layout(constraints).unwrap();
        let layout = tree.text_layout(render).unwrap();
        assert_eq!(tree.element_bounds(field).unwrap().size.height, height, "cap {max:?}");
        assert_eq!(layout.metrics.size.height, height);
        assert_eq!(layout.overflowed, max.is_some());
    }
}

#[test]
fn laid_out_paragraph_maps_an_exact_glyph_boundary_to_the_next_visual_glyph() {
    use incular_text::TextAffinity::{Downstream, Upstream};
    for (source, direction, to_left, at_or_right) in [
        ("1234567", TextDirection::Ltr, Upstream, Downstream),
        ("אבגדהוז", TextDirection::Rtl, Downstream, Upstream),
    ] {
        let Some((style, engine)) = flutter_test_style() else {
            return;
        };
        let key = GlobalKey::new();
        let span = TextSpan::new(source).style(style.font_size(14.));
        let paragraph = Widget::from(RichText::new(span)).with_key(key);
        let mut tree = WidgetTree::with_text_engine(engine);
        tree.mount(Directionality::new(direction, paragraph).into()).unwrap();
        tree.layout(Constraints::loose(Size::new(800., 600.))).unwrap();
        let field = tree.element_with_key(&key.into()).unwrap();
        let start = source.char_indices().nth(3).unwrap().0;
        let end = source.char_indices().nth(4).unwrap().0;
        let boxes = tree.paragraph_selection_boxes(field, TextRange::new(start, end)).unwrap();
        let [rect] = boxes.as_slice() else {
            panic!("expected a single box for the boundary glyph");
        };
        let boundary = if direction == TextDirection::Ltr {
            assert_eq!(rect.rect.origin.x, 42.);
            rect.rect.origin.x
        } else {
            rect.rect.origin.x + rect.rect.size.width
        };
        for (dx, affinity) in [(-1., to_left), (0., at_or_right), (1., at_or_right)] {
            assert_eq!(
                tree.selectable_text_affinity_position_at(field, Offset::new(boundary + dx, 14.)),
                Some((start, affinity)),
                "{direction:?}, boundary {boundary}, delta {dx}",
            );
        }
    }
}

#[test]
fn a_mounted_paragraph_defaults_to_manual_and_relayouts_only_when_hyphens_change() {
    use incular_text::Hyphens;
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::new("abc\u{00ad}de").style(style.font_size(10.));
    let initial = RichText::new(span.clone());
    assert_eq!(initial.hyphens, Hyphens::Manual);
    let (mut tree, key) = mount(span, engine, 49., false);
    let field = tree.element_with_key(&key.into()).unwrap();
    let render = tree.render_id(field).unwrap();
    let constraints = Constraints::tight(Size::new(49., 600.));
    for (mode, expected_glyphs, dirty) in [
        (Hyphens::Manual, 4, false),
        (Hyphens::Hidden, 3, true),
        (Hyphens::Manual, 4, true),
    ] {
        let old_layouts = tree.diagnostics().layouts;
        tree.update(field, Widget::from(initial.clone().hyphens(mode)).with_key(key))
            .unwrap();
        tree.layout(constraints).unwrap();
        assert_eq!(tree.diagnostics().layouts > old_layouts, dirty, "{mode:?}");
        let layout = tree.text_layout(render).unwrap();
        assert_eq!(layout.metrics.size.height, 20.);
        assert_eq!(layout.lines[0].width, 30.);
        assert_eq!(
            layout.lines[0].glyphs.iter().filter(|g| g.advance > 0.).count(),
            expected_glyphs,
            "{mode:?}",
        );
        let mut painted_mark = false;
        let painted = tree.paint();
        for command in painted.commands() {
            if let incular_rendering::PaintCommand::GlyphRun { run, .. } = command {
                painted_mark |= run.glyphs.iter().any(|g| {
                    g.cluster == 3 && g.advance > 0. && g.offset.x + run.origin.x == 30.
                });
            }
        }
        assert_eq!(painted_mark, mode == Hyphens::Manual);
    }
}

#[test]
fn ordinary_and_selectable_text_forward_the_discretionary_hyphen_policy() {
    use incular_widgets::{Hyphens, Text};

    for (selectable, mode, glyphs) in [
        (false, None, 3),
        (false, Some(Hyphens::Hidden), 2),
        (true, Some(Hyphens::Hidden), 2),
        (true, Some(Hyphens::Manual), 3),
    ] {
        let Some((style, engine)) = flutter_test_style() else {
            return;
        };
        let text = Text::new("hy\u{00ad}phenation").style(style.font_size(10.));
        let text = mode.map_or_else(|| text.clone(), |mode| text.clone().hyphens(mode));
        let key = GlobalKey::new();
        let widget = Widget::from(text).with_key(key);
        let root = if selectable {
            Widget::selection_area(SelectionAreaController::new(), widget)
        } else {
            widget
        };
        let root = Directionality::new(TextDirection::Ltr, root);
        let mut tree = WidgetTree::with_text_engine(engine);
        tree.mount(root.into()).unwrap();
        tree.layout(Constraints::loose(Size::new(90., 100.))).unwrap();
        let field = tree.element_with_key(&key.into()).unwrap();
        assert_eq!(tree.is_selectable_text(field), selectable);
        let layout = tree.text_layout(tree.render_id(field).unwrap()).unwrap();
        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.lines[0].width, 20.);
        assert_eq!(
            layout.lines[0].glyphs.iter().filter(|g| g.advance > 0.).count(),
            glyphs,
            "selectable={selectable}, hyphens={mode:?}",
        );
    }
}

#[test]
fn max_selection_boxes_preserve_shaped_runs_and_add_only_the_intermediate_line_extension() {
    use incular_text::{BoxHeightStyle, BoxWidthStyle};

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::new("First ")
        .style(style.font_size(10.))
        .child(TextSpan::new("smallsecond ").style(TextStyle::new().font_size(8.)))
        .child(TextSpan::new("third fourth fifth"));
    let (tree, key) = mount(span, engine, 160., false);
    let element = tree.element_with_key(&key.into()).unwrap();
    let boxes = |range, width| {
        tree.paragraph_selection_boxes_with_style(element, range, BoxHeightStyle::Max, width)
            .unwrap()
            .into_iter()
            .map(|box_| {
                assert_eq!(box_.direction, TextDirection::Ltr);
                edges(box_.rect)
            })
            .collect::<Vec<_>>()
    };
    let range = TextRange::new(0, 36);
    assert_eq!(
        boxes(range, BoxWidthStyle::Max),
        [
            [0., 0., 60., 10.],
            [60., 0., 156., 10.],
            [0., 10., 130., 20.],
            [130., 10., 156., 20.],
            [0., 20., 50., 30.],
        ],
    );
    assert_eq!(boxes(range, BoxWidthStyle::Tight).len(), 4);
    assert_eq!(
        boxes(TextRange::new(18, 21), BoxWidthStyle::Max),
        [[0., 10., 30., 20.]],
    );
    assert!(boxes(TextRange::collapsed(18), BoxWidthStyle::Max).is_empty());
}

#[test]
fn inline_selection_keeps_adjacent_placeholders_separate_with_tight_or_max_height() {
    use incular_text::{BoxHeightStyle, BoxWidthStyle};
    use incular_widgets::{RichParagraph, SizedBox, WidgetSpan};

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::new("a")
        .style(style.font_size(10.))
        .child(WidgetSpan::new(1))
        .child(WidgetSpan::new(2))
        .child(TextSpan::new("a"))
        .child(WidgetSpan::new(3));
    let source_end = span.plain_text().len();
    let key = GlobalKey::new();
    let content = RichParagraph::new(RichText::new(span))
        .child(1, SizedBox::square(14.))
        .child(2, SizedBox::square(14.))
        .child(3, SizedBox::square(14.));
    let node = Widget::from(content).with_key(key);
    let mut tree = WidgetTree::with_text_engine(engine);
    tree.mount(Directionality::new(TextDirection::Ltr, node).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(100., 100.))).unwrap();
    let field = tree.element_with_key(&key.into()).unwrap();
    let range = TextRange::new(0, source_end);
    for (style, text_y) in [(BoxHeightStyle::Tight, 4.), (BoxHeightStyle::Max, 0.)] {
        let boxes = tree
            .paragraph_selection_boxes_with_style(field, range, style, BoxWidthStyle::Tight)
            .unwrap();
        assert_eq!(boxes.len(), 5, "{boxes:?}");
        for (actual, expected) in boxes.into_iter().zip([
            [0., text_y, 10., 14.],
            [10., 0., 24., 14.],
            [24., 0., 38., 14.],
            [38., text_y, 48., 14.],
            [48., 0., 62., 14.],
        ]) {
            assert_eq!(actual.direction, TextDirection::Ltr);
            for (got, want) in edges(actual.rect).into_iter().zip(expected) {
                close(got, want);
            }
        }
    }
}

#[test]
fn tight_editable_selection_through_a_hard_break_stays_on_the_short_line_without_focus() {
    use incular_core::Color;
    use incular_rendering::PaintCommand;
    use incular_text::{BoxHeightStyle, BoxWidthStyle};
    use incular_widgets::EditableText;

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let controller = incular_text::TextEditingController::with_text("abc\nabcdefghij");
    let widget = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .max_lines(None)
        .style(style.font_size(10.).line_height_multiplier(1.))
        .selection_color(Color::BLACK);
    assert_eq!(
        widget.configured_selection_width_style(),
        BoxWidthStyle::Tight
    );
    assert_eq!(
        widget.configured_selection_height_style(),
        if cfg!(target_arch = "wasm32") {
            BoxHeightStyle::Max
        } else {
            BoxHeightStyle::IncludeLineSpacingMiddle
        }
    );
    let widget = widget.selection_width_style(BoxWidthStyle::Tight);
    let mut tree = WidgetTree::with_text_engine(engine);
    let field = tree.mount(widget.into()).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.layout(constraints).unwrap();
    assert_eq!(tree.focused_keyboard_element(), None);
    controller.set_selection(TextSelection::new(0, 4));
    tree.layout(constraints).unwrap();
    let selection: Vec<_> = tree.paint().commands().iter().filter_map(|command| match command {
        PaintCommand::Rect { rect, color: Color::BLACK } => Some(edges(*rect)),
        _ => None,
    }).collect();
    assert_eq!(selection, [[0., 0., 30., 10.]]);
    assert_eq!(tree.focused_keyboard_element(), None);
    assert_eq!(tree.root(), Some(field));
}

#[test]
fn retained_editable_width_style_repaints_whole_and_segmented_layouts_without_remeasuring() {
    use incular_core::Color;
    use incular_rendering::PaintCommand;
    use incular_text::{BoxHeightStyle, BoxWidthStyle};
    use incular_widgets::EditableText;

    for segmented in [false, true] {
        let Some((style, engine)) = flutter_test_style() else {
            return;
        };
        let controller = incular_text::TextEditingController::with_text("abc\nabcdefghij");
        controller.set_selection(TextSelection::new(0, 4));
        let build = |height, width| {
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .max_lines(None)
                .paragraph_layout(segmented)
                .style(style.clone().font_size(10.).line_height_multiplier(1.))
                .selection_color(Color::BLACK)
                .selection_height_style(height)
                .selection_width_style(width)
        };
        let constraints = Constraints::tight(Size::new(800., 600.));
        let fixed_height = BoxHeightStyle::IncludeLineSpacingMiddle;
        let mut tree = WidgetTree::with_text_engine(engine);
        let field = tree
            .mount(build(fixed_height, BoxWidthStyle::Tight).into())
            .unwrap();
        tree.layout(constraints).unwrap();
        let render = tree.render_id(field).unwrap();
        let before = tree.diagnostics().layouts;
        for (height, width, expected) in [
            (
                fixed_height,
                BoxWidthStyle::Tight,
                &[[0., 0., 30., 10.]][..],
            ),
            (
                fixed_height,
                BoxWidthStyle::Max,
                &[[0., 0., 30., 10.], [30., 0., 100., 10.]][..],
            ),
            (
                BoxHeightStyle::Max,
                BoxWidthStyle::Tight,
                &[[0., 0., 30., 10.]][..],
            ),
        ] {
            tree.update(field, build(height, width).into()).unwrap();
            tree.layout(constraints).unwrap();
            let selected: Vec<_> = tree
                .paint()
                .commands()
                .iter()
                .filter_map(|command| match command {
                    PaintCommand::Rect {
                        rect,
                        color: Color::BLACK,
                    } => Some(edges(*rect)),
                    _ => None,
                })
                .collect();
            assert_eq!(
                selected, expected,
                "segmented: {segmented}, {height:?}, {width:?}"
            );
            assert_eq!(tree.render_id(field), Some(render));
            assert_eq!(
                tree.diagnostics().layouts,
                before,
                "a selection style is paint-only"
            );
        }
    }
}

#[test]
fn editable_default_height_covers_middle_line_spacing_across_entire_document() {
    use incular_core::Color;
    use incular_rendering::PaintCommand;
    use incular_text::BoxHeightStyle;
    use incular_widgets::EditableText;

    for segmented in [false, true] {
        let Some((style, engine)) = flutter_test_style() else {
            return;
        };
        let controller = incular_text::TextEditingController::with_text("ab\ncd\nef");
        controller.set_selection(TextSelection::new(0, 8));
        let build = || {
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .max_lines(None)
                .paragraph_layout(segmented)
                .style(style.clone().font_size(10.).line_height_multiplier(2.))
                .selection_color(Color::BLACK)
        };
        let constraints = Constraints::tight(Size::new(800., 600.));
        let mut tree = WidgetTree::with_text_engine(engine);
        let field = tree.mount(build().into()).unwrap();
        tree.layout(constraints).unwrap();
        let boxes = |tree: &mut WidgetTree| -> Vec<_> {
            tree.paint()
                .commands()
                .iter()
                .filter_map(|command| match command {
                    PaintCommand::Rect {
                        rect,
                        color: Color::BLACK,
                    } => Some(edges(*rect)),
                    _ => None,
                })
                .collect()
        };
        let max = [[0., 0., 20., 20.], [0., 20., 20., 40.], [0., 40., 20., 60.]];
        if cfg!(target_arch = "wasm32") {
            assert_eq!(boxes(&mut tree), max);
        } else {
            assert_eq!(
                boxes(&mut tree),
                [
                    [0., 7.5, 20., 23.75],
                    [0., 23.75, 20., 43.75],
                    [0., 43.75, 20., 60.]
                ],
                "segmented: {segmented}",
            );
        }
        let before = tree.diagnostics().layouts;
        tree.update(
            field,
            build().selection_height_style(BoxHeightStyle::Max).into(),
        )
        .unwrap();
        tree.layout(constraints).unwrap();
        assert_eq!(boxes(&mut tree), max, "segmented: {segmented}");
        assert_eq!(tree.diagnostics().layouts, before);
    }
}

#[test]
fn retained_paragraph_overflow_modes_resize_and_scope_the_fade() {
    use incular_rendering::PaintCommand;
    use incular_text::TextOverflow;
    use incular_widgets::Text;

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let paragraph_key = GlobalKey::new();
    let constraints = Constraints::loose(Size::new(50., 600.));
    let build = |max_lines, soft_wrap, overflow| {
        Widget::from(
            Text::new(concat!(
                "This\n",
                "is a wrapping test. It should wrap at manual newlines, ",
                "and if softWrap is true, also at spaces.",
            ))
            .text_direction(TextDirection::Ltr)
            .style(style.clone().font_size(10.))
            .max_lines(max_lines)
            .soft_wrap(soft_wrap)
            .overflow(overflow),
        )
        .with_key(paragraph_key)
    };

    let mut tree = WidgetTree::with_text_engine(engine);
    let root = tree
        .mount(build(Some(1), true, TextOverflow::Clip))
        .unwrap();
    tree.layout(constraints).unwrap();
    let paragraph = tree.element_with_key(&paragraph_key.into()).unwrap();
    let line = tree.element_bounds(paragraph).unwrap().size.height;
    close(line, 10.);

    for (max_lines, soft_wrap, overflow, expected, fade) in [
        (Some(3), true, TextOverflow::Clip, Some(3.), false),
        (None, true, TextOverflow::Clip, None, false),
        (Some(1), true, TextOverflow::Ellipsis, Some(1.), false),
        (Some(3), true, TextOverflow::Ellipsis, Some(3.), false),
        (None, true, TextOverflow::Ellipsis, Some(2.), false),
        (Some(1), false, TextOverflow::Clip, Some(1.), false),
        (Some(3), false, TextOverflow::Clip, Some(2.), false),
        (None, false, TextOverflow::Clip, Some(2.), false),
        (Some(1), false, TextOverflow::Ellipsis, Some(1.), false),
        (Some(3), false, TextOverflow::Ellipsis, Some(3.), false),
        (None, false, TextOverflow::Ellipsis, Some(2.), false),
        (Some(3), true, TextOverflow::Fade, Some(3.), true),
        (Some(3), true, TextOverflow::Ellipsis, Some(3.), false),
        (Some(100), true, TextOverflow::Fade, None, false),
    ] {
        tree.update(root, build(max_lines, soft_wrap, overflow))
            .unwrap();
        tree.layout(constraints).unwrap();
        assert_eq!(
            tree.element_with_key(&paragraph_key.into()),
            Some(paragraph)
        );
        let height = tree.element_bounds(paragraph).unwrap().size.height;
        if let Some(expected) = expected {
            assert!(
                (height - expected * line).abs() < 0.001,
                "{max_lines:?} {soft_wrap} {overflow:?}: {height} expected {}",
                expected * line,
            );
        } else {
            assert!(height > 5. * line, "unlimited wraps preserve all lines");
        }
        let list = tree.paint();
        assert_eq!(
            list.commands()
                .iter()
                .any(|op| matches!(op, PaintCommand::PushShaderMask { .. })),
            fade,
            "{max_lines:?} {soft_wrap} {overflow:?}",
        );
    }
}

#[test]
fn no_wrap_paragraph_fades_toward_the_physical_trailing_edge() {
    use incular_rendering::{BlendMode, Brush, PaintCommand};
    use incular_text::TextOverflow;
    use incular_widgets::Text;

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let fit = Constraints::loose(Size::new(200., 40.));
    let narrow = Constraints::loose(Size::new(50., 40.));
    let build = |direction| {
        Widget::from(
            Text::new("ABCDEFGHIJK")
                .text_direction(direction)
                .style(style.clone().font_size(10.))
                .soft_wrap(false)
                .overflow(TextOverflow::Fade),
        )
    };
    let mut tree = WidgetTree::with_text_engine(engine);
    let root = tree.mount(build(TextDirection::Ltr)).unwrap();
    for (direction, start, end) in [
        (
            TextDirection::Ltr,
            Offset::new(40., 0.),
            Offset::new(50., 0.),
        ),
        (TextDirection::Rtl, Offset::new(10., 0.), Offset::ZERO),
    ] {
        tree.update(root, build(direction)).unwrap();
        tree.layout(narrow).unwrap();
        let painted = tree.paint();
        let masks: Vec<_> = painted
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushShaderMask {
                    shader: Brush::LinearGradient(brush),
                    blend_mode,
                    mask_size,
                    ..
                } => Some((brush, *blend_mode, *mask_size)),
                _ => None,
            })
            .collect();
        assert_eq!(masks.len(), 1);
        let (gradient, mode, size) = masks[0];
        assert_eq!(mode, BlendMode::Modulate);
        assert_eq!(size, Size::new(50., 10.));
        assert_eq!(gradient.start, start);
        assert_eq!(gradient.end, end);
    }
    tree.layout(fit).unwrap();
    assert!(
        !tree
            .paint()
            .commands()
            .iter()
            .any(|op| matches!(op, PaintCommand::PushShaderMask { .. }))
    );
}

#[test]
fn paragraph_fade_keeps_selection_outside_and_paints_inline_widgets_inside() {
    use incular_core::Color;
    use incular_rendering::{Brush, PaintCommand};
    use incular_text::TextOverflow;
    use incular_widgets::{RichParagraph, WidgetSpan};

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::new("A")
        .style(style.font_size(10.))
        .child(WidgetSpan::new(7))
        .child(TextSpan::new("\nB\nC\nD"));
    let marker = Color::rgba(21, 190, 87, 255);
    let selected = Color::rgba(72, 120, 220, 150);
    let key = GlobalKey::new();
    let paragraph = Widget::from(
        RichParagraph::new(
            RichText::new(span)
                .max_lines(Some(3))
                .overflow(TextOverflow::Fade),
        )
        .child(7, Widget::box_(Size::new(10., 10.), marker)),
    )
    .with_key(key);
    let controller = SelectionAreaController::new();
    let scene = Directionality::new(
        TextDirection::Ltr,
        Widget::selection_area(controller, paragraph),
    );
    let mut tree = WidgetTree::with_text_engine(engine);
    tree.mount(scene.into()).unwrap();
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .unwrap();
    let label = tree.element_with_key(&key.into()).unwrap();
    assert!(tree.selectable_text_select_all(label));
    let commands = tree.paint();
    let commands = commands.commands();
    let mask = commands
        .iter()
        .position(|command| {
            matches!(
                command,
                PaintCommand::PushShaderMask { shader: Brush::LinearGradient(brush), .. }
                    if brush.start == Offset::new(0., 25.) && brush.end == Offset::new(0., 30.)
            )
        })
        .expect("three 10px lines fade over half an ellipsis line");
    let pop = mask
        + commands[mask..]
            .iter()
            .position(|command| matches!(command, PaintCommand::PopEffect))
            .unwrap();
    let highlight = |command: &PaintCommand| matches!(command, PaintCommand::Rect { color, .. } if *color == selected);
    assert!(commands[..mask].iter().any(highlight));
    assert!(!commands[mask..].iter().any(highlight));
    assert!(
        commands[mask..pop]
            .iter()
            .any(|command| matches!(command, PaintCommand::GlyphRun { .. }))
    );
    assert!(
        commands[mask..pop]
            .iter()
            .any(|command| matches!(command, PaintCommand::Rect { color, .. } if *color == marker))
    );
}

fn literal_span_targets(tree: &WidgetTree, x: f32, y: f32) -> Vec<String> {
    tree.pointer_hit_test(Offset::new(x, y))
        .text_span_targets()
        .map(|span| span.text.clone())
        .collect()
}

#[test]
fn paragraph_hit_testing_keeps_the_whole_letter_spaced_grapheme_advance() {
    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::empty()
        .style(style.font_size(10.))
        .child(TextSpan::new("AAAAAAAAAA"))
        .child(TextSpan::new("BC").style(TextStyle::default().letter_spacing(26.)));
    let (tree, _) = mount(span, engine, 100., false);
    for (x, y, expected) in [
        (5., 5., Some("AAAAAAAAAA")),
        (95., 5., Some("AAAAAAAAAA")),
        (200., 5., None),
        (18., 15., Some("BC")),
        (31., 15., Some("BC")),
        (54., 15., Some("BC")),
        (100., 15., None),
        (9999., 9999., None),
    ] {
        assert_eq!(
            literal_span_targets(&tree, x, y),
            expected.into_iter().collect::<Vec<_>>(),
            "hit at ({x}, {y})"
        );
    }
}

#[test]
fn justified_space_is_hit_as_part_of_its_own_horizontal_text_span() {
    use incular_text::TextAlign;

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let span = TextSpan::empty()
        .style(style.font_size(10.))
        .child(TextSpan::new("A "))
        .child(TextSpan::new("B\u{200b}"))
        .child(TextSpan::new("CCCCCCCCCC"));
    let paragraph = RichText::new(span).text_align(TextAlign::Justify);
    let mut tree = WidgetTree::with_text_engine(engine);
    tree.mount(Directionality::new(TextDirection::Ltr, paragraph).into())
        .unwrap();
    tree.layout(Constraints::new(100., 100., 0., 600.)).unwrap();
    assert_eq!(literal_span_targets(&tree, 5., 5.), ["A "]);
    assert_eq!(literal_span_targets(&tree, 50., 5.), ["A "]);
    assert_eq!(literal_span_targets(&tree, 95., 5.), ["B\u{200b}"]);
}

#[test]
fn the_paragraph_locale_setter_updates_the_existing_retained_text() {
    use incular_config::Locale;
    use incular_widgets::Text;

    let mut tree = WidgetTree::new();
    let make = |locale: &str| {
        Widget::from(
            Text::new("Hello")
                .text_direction(TextDirection::Ltr)
                .locale(locale.parse::<Locale>().unwrap()),
        )
    };
    let root = tree.mount(make("zh-HK")).unwrap();
    let render = tree.render_id(root).unwrap();
    let constraints = Constraints::loose(Size::new(100., 100.));
    tree.layout(constraints).unwrap();
    assert_eq!(tree.paragraph_locale(root), Some("zh-HK"));
    tree.update(root, make("ja-JP")).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.render_id(root), Some(render));
    assert_eq!(tree.paragraph_locale(root), Some("ja-JP"));
}

#[test]
fn inline_selection_keeps_each_of_nine_boxes_across_three_wrapped_lines() {
    use incular_widgets::{RichParagraph, SizedBox, WidgetSpan};

    let Some((style, engine)) = flutter_test_style() else {
        return;
    };
    let mut span = TextSpan::new("a")
        .style(style.font_size(10.))
        .child(WidgetSpan::new(1))
        .child(WidgetSpan::new(2))
        .child(TextSpan::new("a"));
    for id in 3..=7 {
        span = span.child(WidgetSpan::new(id));
    }
    let end = span.plain_text().len();
    let mut paragraph = RichParagraph::new(RichText::new(span));
    for id in 1..=7 {
        paragraph = paragraph.child(id, SizedBox::square(14.));
    }
    let key = GlobalKey::new();
    let mut tree = WidgetTree::with_text_engine(engine);
    tree.mount(
        Directionality::new(TextDirection::Ltr, Widget::from(paragraph).with_key(key)).into(),
    )
    .unwrap();
    tree.layout(Constraints::loose(Size::new(50., 100.)))
        .unwrap();
    let element = tree.element_with_key(&key.into()).unwrap();
    let boxes = tree
        .paragraph_selection_boxes(element, TextRange::new(0, end))
        .unwrap();
    let expected = [
        [0., 4., 10., 14.],
        [10., 0., 24., 14.],
        [24., 0., 38., 14.],
        [38., 4., 48., 14.],
        [0., 14., 14., 28.],
        [14., 14., 28., 28.],
        [28., 14., 42., 28.],
        [0., 28., 14., 42.],
        [14., 28., 28., 42.],
    ];
    assert_eq!(boxes.len(), expected.len());
    for (actual, expected) in boxes.into_iter().zip(expected) {
        assert_eq!(actual.direction, TextDirection::Ltr);
        for (actual, expected) in edges(actual.rect).into_iter().zip(expected) {
            close(actual, expected);
        }
    }
}
