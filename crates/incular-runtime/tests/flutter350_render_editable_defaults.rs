use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::{Offset, Size};
use incular_runtime::Runtime;
use incular_semantics::{SemanticRole, SemanticsInputType};
use incular_text::{TextEditingController, TextSelection, TextStyle, register_application_font_as};
use incular_widgets::{EditableText, GlobalKey, Padding, Widget};

const FAMILY: &str = "RenderEditable Pinned Ahem";

fn ahem_style(size: f32) -> Option<TextStyle> {
    let path = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../.tmp/flutter/packages/flutter_tools/static/Ahem.ttf")
        });
    if !path.is_file() {
        eprintln!(
            "Pinned Flutter Ahem missing at {}; geometry comparison not run",
            path.display()
        );
        return None;
    }
    register_application_font_as(FAMILY, std::fs::read(path).unwrap());
    Some(TextStyle::new().font_family(FAMILY).font_size(size))
}

fn editor(text: impl Into<String>) -> EditableText {
    EditableText::new(TextEditingController::with_text(text)).text_direction(TextDirection::Ltr)
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::loose(Size::new(600., 600.)))
        .unwrap();
}

#[test]
fn an_unfocused_plain_editor_reports_default_text_input_semantics() {
    let mut runtime = Runtime::new(editor("text").into()).unwrap();
    frame(&mut runtime);
    assert!(runtime.focused_element().is_none());
    let semantics = runtime.tree().semantics();
    let [id] = semantics.roots() else {
        panic!("an ordinary editor should produce exactly one semantic root");
    };
    let semantic = semantics.node(*id).unwrap();
    assert_eq!(semantic.role, SemanticRole::TextField);
    assert_eq!(semantic.state.input_type, SemanticsInputType::Text);
}

#[test]
fn default_single_line_editor_reports_first_line_height_even_for_many_hard_breaks() {
    let Some(style) = ahem_style(10.) else {
        return;
    };
    let mut runtime = Runtime::new(editor("liiiiines\n".repeat(10)).style(style).into()).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    assert_eq!(
        runtime.tree().element_bounds(root).unwrap().size.height,
        10.
    );
}

#[test]
fn a_uniform_tall_line_retains_its_full_height_in_the_default_single_line_editor() {
    let Some(style) = ahem_style(100.) else {
        return;
    };
    let mut runtime = Runtime::new(editor("TALL").style(style).into()).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    assert_eq!(
        runtime.tree().element_bounds(root).unwrap().size.height,
        100.
    );
}

#[test]
fn translated_raw_editor_moves_its_collapsed_selection_anchor_by_the_paint_offset() {
    let Some(style) = ahem_style(20.) else {
        return;
    };
    let anchor = |offset: Offset| {
        let controller = TextEditingController::with_text("text");
        controller.set_selection(TextSelection::collapsed(0));
        let key = GlobalKey::new();
        let editor = Widget::from(
            EditableText::new(controller)
                .text_direction(TextDirection::Ltr)
                .cursor_width(1.)
                .style(style.clone().height(Some(1.))),
        )
        .with_key(key);
        let scene = Padding::new(EdgeInsets::only(offset.x, offset.y, 0., 0.), editor);
        let mut runtime = Runtime::new(scene.into()).unwrap();
        frame(&mut runtime);
        let id = runtime.tree().element_with_key(&key.into()).unwrap();
        let handles = runtime
            .tree()
            .text_field_selection_handle_geometry(id)
            .unwrap();
        assert!(handles.end.is_none());
        handles.start.anchor
    };
    assert_eq!(anchor(Offset::ZERO), Offset::new(0., 20.));
    assert_eq!(anchor(Offset::new(100., 200.)), Offset::new(100., 220.));
}

#[test]
fn a_mounted_caret_can_change_color_radius_width_text_scale_and_visibility() {
    let Some(style) = ahem_style(10.) else {
        return;
    };
    check_live_caret_configuration(style.height(Some(1.)), "test", 4);
}

fn flutter_test_style(family: &str, size: f32) -> Option<TextStyle> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.tmp/flutter/engine/src/flutter/runtime/test_font_data.cc");
    let Ok(source) = std::fs::read_to_string(path) else {
        eprintln!("Pinned engine test font unavailable; ideographic font comparison not run");
        return None;
    };
    // Extract only the reference engine's embedded FlutterTest font, not a
    // system CJK font with different advances, BASE tables or fallback.
    let encoded = source
        .split_once("static const unsigned char kFlutterTestFont[] = {")
        .and_then(|(_, suffix)| suffix.split_once("};"))
        .expect("pinned engine embeds the FlutterTest font")
        .0;
    let font = encoded
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| u8::from_str_radix(part.strip_prefix("0x").unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(font.len(), 2984);
    register_application_font_as(family, font);
    Some(TextStyle::new().font_family(family).font_size(size))
}

#[test]
fn a_mounted_caret_uses_ideographic_glyphs_at_the_same_scaled_positions() {
    let Some(style) = flutter_test_style("Pinned FlutterTest ideographic caret", 10.) else {
        return;
    };
    let text = "中文测试文本是否正确";
    // Flutter's offset 4 is UTF-16; Incular stores an equivalent UTF-8
    // caret at the start of the fifth glyph.
    let offset = text.char_indices().nth(4).unwrap().0;
    assert_eq!(offset, 12);
    check_live_caret_configuration(style, text, offset);
}

fn check_live_caret_configuration(style: TextStyle, text: &str, offset: usize) {
    use incular_core::{Color, Rect};
    use incular_rendering::{Brush, DisplayList, PaintCommand, RRect};
    use incular_text::{TextAffinity, TextScaler};

    let controller = TextEditingController::with_text(text);
    controller
        .set_selection_with_affinity(TextSelection::collapsed(offset), TextAffinity::Upstream);

    const RED: Color = Color::rgba(255, 0, 0, 255);
    const BLUE: Color = Color::rgba(0, 0, 255, 255);
    let build = |visible, color, width, radius, scale| {
        EditableText::new(controller.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .style(style.clone())
            .text_scaler(TextScaler::linear(scale))
            .cursor_color(color)
            .cursor_width(width)
            .cursor_radius(radius)
            .show_cursor(visible)
    };
    let mut runtime = Runtime::new(build(false, RED, 1., 0., 1.).into()).unwrap();
    runtime.set_debug_deterministic_cursor(true);
    let frame = |runtime: &mut Runtime| {
        runtime
            .run_frame(Constraints::loose(Size::new(100., 100.)))
            .unwrap()
            .0
    };

    let rectangles = |list: &DisplayList| {
        list.commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Rect { rect, color } if *color == RED || *color == BLUE => {
                    Some((*rect, *color))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let rounded = |list: &DisplayList| {
        list.commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::RRect {
                    rrect,
                    brush: Brush::Solid(color),
                } if *color == RED || *color == BLUE => Some((*rrect, *color)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let hidden = frame(&mut runtime);
    assert!(rectangles(&hidden).is_empty());
    assert!(rounded(&hidden).is_empty());
    let field = runtime.focused_element().unwrap();

    let rect = |x, w, h| Rect::from_origin_size(Offset::new(x, 0.), Size::new(w, h));
    let mut transition = |widget: EditableText| {
        runtime.schedule_update(field, widget.into()).unwrap();
        let list = frame(&mut runtime);
        assert_eq!(runtime.focused_element(), Some(field));
        assert_eq!(controller.selection(), TextSelection::collapsed(offset));
        assert_eq!(controller.caret_affinity(), TextAffinity::Upstream);
        list
    };

    let plain = transition(build(true, RED, 1., 0., 1.));
    assert_eq!(rectangles(&plain), [(rect(40., 1., 10.), RED)]);
    assert!(rounded(&plain).is_empty());
    let round = transition(build(true, BLUE, 4., 3., 1.));
    assert!(rectangles(&round).is_empty());
    assert_eq!(
        rounded(&round),
        [(RRect::uniform(rect(40., 4., 10.), 3.), BLUE)]
    );
    let doubled = transition(build(true, BLUE, 4., 3., 2.));
    assert!(rectangles(&doubled).is_empty());
    assert_eq!(
        rounded(&doubled),
        [(RRect::uniform(rect(80., 4., 20.), 3.), BLUE)]
    );
    let hidden_again = transition(build(false, BLUE, 4., 3., 2.));
    assert!(rectangles(&hidden_again).is_empty());
    assert!(rounded(&hidden_again).is_empty());
}

#[test]
fn the_rendered_floating_prototype_ignores_the_text_viewport_offset() {
    use incular_config::TargetPlatform;
    use incular_core::{Color, Rect, Size};
    use incular_rendering::{Brush, DisplayList, PaintCommand, RRect};
    use incular_text::TextAffinity;

    const RED: Color = Color::rgba(255, 0, 0, 255);
    let Some(style) = ahem_style(10.) else {
        return;
    };
    let controller = TextEditingController::with_text("test");
    controller.set_selection_with_affinity(TextSelection::collapsed(4), TextAffinity::Upstream);
    let build = |show, scroll| {
        EditableText::new(controller.clone())
            .style(style.clone().height(Some(1.)))
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .max_lines(Some(3))
            .fixed_scroll_offset(scroll)
            .cursor_color(RED)
            .cursor_width(1.)
            .show_cursor(show)
    };
    let constraints = Constraints::loose(Size::new(100., 100.));
    let mut runtime = Runtime::new(build(false, 0.).into()).unwrap();
    runtime.set_debug_deterministic_cursor(true);
    let (initial, _) = runtime.run_frame(constraints).unwrap();
    assert!(
        !initial.commands().iter().any(|command| {
            matches!(command, PaintCommand::Rect { color, .. } if *color == RED)
        })
    );
    let field = runtime.focused_element().unwrap();
    let affinity = TextAffinity::Upstream;
    let floating = |list: &DisplayList| {
        list.commands().iter().find_map(|command| match command {
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } if *color == Color { alpha: 191, ..RED } => Some(*rrect),
            _ => None,
        })
    };
    let (y, height) = if TargetPlatform::current().is_apple() {
        (49., 14.)
    } else {
        (51., 8.)
    };
    let expected = RRect::uniform(
        Rect::from_origin_size(Offset::new(49.5, y), Size::new(2., height)),
        1.,
    );
    let mut starting_unscrolled_ordinary_y = None;
    for scroll in [0., 200.] {
        runtime
            .schedule_update(field, build(true, scroll).into())
            .unwrap();
        runtime.run_frame(constraints).unwrap();
        let geometry = runtime
            .tree()
            .floating_caret_geometry(field, 4, affinity)
            .unwrap();
        let ordinary_y = geometry.caret.origin.y;
        let starting_y = *starting_unscrolled_ordinary_y.get_or_insert(ordinary_y);
        assert_eq!(ordinary_y, starting_y - scroll);
        let rect = geometry.floating_rect_at(Offset::new(50., 50.), 1.);
        runtime
            .tree_mut()
            .set_floating_caret_with_anchor(field, Some(rect), Some((4, affinity)));
        let (list, _) = runtime.run_frame(constraints).unwrap();
        assert_eq!(floating(&list), Some(expected));
    }
}

#[test]
fn disabling_force_line_shrinks_the_mounted_editor_and_respects_parent_constraints() {
    use incular_text::TextAlign;

    let Some(style) = ahem_style(10.) else {
        return;
    };
    let controller = TextEditingController::with_text("A");
    controller.set_selection(TextSelection::collapsed(0));
    let build = |force_line: Option<bool>, align| {
        let editor = EditableText::new(controller.clone())
            .style(style.clone().height(Some(1.)))
            .text_direction(TextDirection::Ltr)
            .cursor_width(2.)
            .fixed_scroll_offset(10.)
            .text_align(align);
        match force_line {
            Some(force) => editor.force_line(force),
            None => editor,
        }
    };
    let mut runtime = Runtime::new(build(None, TextAlign::Start).into()).unwrap();
    let root = runtime.tree().root().unwrap();
    let constrained = Constraints::new(100., 500., 0., 100.);
    let frame = |runtime: &mut Runtime, constraints| {
        runtime.run_frame(constraints).unwrap();
        runtime.tree().root_layout_size().unwrap()
    };
    assert_eq!(frame(&mut runtime, constrained), Size::new(500., 10.));
    runtime
        .schedule_update(root, build(Some(false), TextAlign::Start).into())
        .unwrap();
    assert_eq!(frame(&mut runtime, constrained), Size::new(103., 10.));
    assert_eq!(
        frame(&mut runtime, Constraints::loose(Size::new(500., 100.))),
        Size::new(13., 10.),
    );
    assert_eq!(
        frame(&mut runtime, Constraints::unbounded()),
        Size::new(13., 10.),
    );
    assert_eq!(
        frame(&mut runtime, Constraints::tight(Size::new(130., 10.))),
        Size::new(130., 10.),
    );
    assert_eq!(controller.selection(), TextSelection::collapsed(0));
    runtime
        .schedule_update(root, build(Some(false), TextAlign::Center).into())
        .unwrap();
    let loose = Constraints::loose(Size::new(500., 100.));
    assert_eq!(frame(&mut runtime, loose), Size::new(13., 10.));
    let caret = runtime
        .tree()
        .floating_caret_geometry(root, 0, incular_text::TextAffinity::Downstream)
        .unwrap()
        .caret;
    // Layout uses the shrunk width for alignment before applying the
    // fixed ten-pixel viewport offset.
    assert_eq!(caret.origin.x, -10.);
    runtime
        .schedule_update(root, build(Some(true), TextAlign::Start).into())
        .unwrap();
    assert_eq!(frame(&mut runtime, constrained), Size::new(500., 10.));
}

#[test]
fn force_line_and_width_basis_keep_distinct_widths_after_soft_wraps() {
    use incular_text::TextWidthBasis;

    let Some(style) = ahem_style(10.) else {
        return;
    };
    let controller = TextEditingController::with_text("AA BB CC");
    let constraints = Constraints::loose(Size::new(50., 100.));
    for paragraphs in [false, true] {
        let editor = || {
            EditableText::new(controller.clone())
                .style(style.clone().height(Some(1.)))
                .text_direction(TextDirection::Ltr)
                .cursor_width(2.)
                .max_lines(None)
                .paragraph_layout(paragraphs)
                .force_line(false)
        };
        let mut runtime = Runtime::new(editor().into()).unwrap();
        let root = runtime.tree().root().unwrap();
        let frame = |runtime: &mut Runtime| {
            runtime.run_frame(constraints).unwrap();
            runtime.tree().root_layout_size().unwrap()
        };
        assert_eq!(frame(&mut runtime), Size::new(50., 30.));
        runtime
            .schedule_update(
                root,
                editor()
                    .text_width_basis(TextWidthBasis::LongestLine)
                    .into(),
            )
            .unwrap();
        assert_eq!(frame(&mut runtime), Size::new(23., 30.));
        runtime
            .schedule_update(
                root,
                editor()
                    .text_width_basis(TextWidthBasis::LongestLine)
                    .force_line(true)
                    .into(),
            )
            .unwrap();
        assert_eq!(frame(&mut runtime), Size::new(50., 30.));
        assert_eq!(controller.text(), "AA BB CC");
    }
}

#[test]
fn raw_editor_mounts_with_the_default_cursor_alignment_and_keyboard_traits() {
    use incular_core::Color;
    use incular_platform::{TextInputCommand, TextInputType};

    let controller = TextEditingController::with_text("name");
    controller.set_selection(TextSelection::collapsed(0));
    let field = EditableText::new(controller)
        .text_direction(TextDirection::Ltr)
        .cursor_color(Color::rgba(33, 150, 243, 255))
        .background_cursor_color(Color::rgba(158, 158, 158, 255))
        .style(TextStyle::new().font_size(14.));
    assert_eq!(field.configured_cursor_width(), 2.);
    assert_eq!(field.configured_cursor_height(), None);
    assert_eq!(field.configured_text_height_behavior(), None);

    let mut runtime = Runtime::new(field.autofocus(true).into()).unwrap();
    frame(&mut runtime);
    let id = runtime.focused_element().unwrap();
    assert_eq!(runtime.tree().text_field_height_behavior(id), Some(None));
    let command = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient {
                configuration,
                caret_rect,
                ..
            } => Some((configuration, caret_rect)),
            _ => None,
        })
        .unwrap();
    let (configuration, caret) = command;
    assert!(!configuration.multiline && !configuration.obscure_text);
    assert_eq!(configuration.input_type, TextInputType::Text);
    assert!(configuration.traits.autocorrect);
    assert!(configuration.traits.enable_suggestions);
    assert!(configuration.traits.enable_ime_personalized_learning);
    assert_eq!(configuration.traits.hint_locales, None);
    assert_eq!(
        caret.origin.x, 0.,
        "the LTR start aligns the caret to the left"
    );
}

#[test]
fn mounted_editor_resolves_inherited_and_explicit_height_behavior_independently() {
    use incular_text::{StrutStyle, TextHeightBehavior};
    use incular_widgets::DefaultTextHeightBehavior;

    let Some(style) = flutter_test_style("Pinned FlutterTest editable height behavior", 20.) else {
        return;
    };
    let inherited = TextHeightBehavior {
        apply_height_to_first_ascent: false,
        ..Default::default()
    };
    let explicit = TextHeightBehavior {
        apply_height_to_last_descent: false,
        ..inherited
    };
    let controller = TextEditingController::with_text("A");
    let key = GlobalKey::new();
    let view = |scope_behavior, override_behavior| {
        let mut field = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .style(style.clone().height(Some(2.)))
            .strut_style(StrutStyle::new().font_size(0.));
        if let Some(behavior) = override_behavior {
            field = field.text_height_behavior(behavior);
        }
        let child = Widget::from(field).with_key(key);
        Widget::from(DefaultTextHeightBehavior::new(scope_behavior, child))
    };
    let mut runtime = Runtime::new(view(inherited, None)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    let render = runtime.tree().render_id(id);
    let mut heights = Vec::new();
    for (scope_behavior, override_behavior, expected) in [
        (inherited, None, inherited),
        (inherited, Some(explicit), explicit),
        (
            TextHeightBehavior::default(),
            None,
            TextHeightBehavior::default(),
        ),
        (inherited, None, inherited),
    ] {
        runtime
            .schedule_update(root, view(scope_behavior, override_behavior))
            .unwrap();
        frame(&mut runtime);
        assert_eq!(runtime.tree().render_id(id), render);
        assert_eq!(
            runtime.tree().text_field_height_behavior(id),
            Some(Some(expected))
        );
        heights.push(runtime.tree().element_bounds(id).unwrap().size.height);
    }
    assert_eq!(heights, [25., 20., 40., 25.]);
}
