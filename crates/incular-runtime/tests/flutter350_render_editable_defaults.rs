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
    use incular_core::{Color, Rect};
    use incular_rendering::{Brush, DisplayList, PaintCommand, RRect};
    use incular_text::TextAffinity;
    use incular_text::TextScaler;

    let Some(style) = ahem_style(10.) else {
        return;
    };
    let style = style.height(Some(1.));
    let controller = TextEditingController::with_text("test");
    controller.set_selection_with_affinity(TextSelection::collapsed(4), TextAffinity::Upstream);

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
        assert_eq!(controller.selection(), TextSelection::collapsed(4));
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
