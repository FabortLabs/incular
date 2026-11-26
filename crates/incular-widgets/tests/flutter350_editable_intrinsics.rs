//! Actual retained editor measurement against the Flutter 3.50.0 Ahem fixture.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_text::{StrutStyle, TextEditingController, TextScaler, TextStyle, register_application_font_as};
use incular_widgets::{EditableText, Widget, internal::WidgetTree};

const FAMILY: &str = "Pinned Editable Ahem";

fn base_style() -> Option<TextStyle> {
    let Ok(font) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return None;
    };
    let bytes = std::fs::read(font).expect("read pinned Ahem");
    register_application_font_as(FAMILY, bytes);
    Some(TextStyle::default().font_family(FAMILY).font_size(10.))
}

fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(400., 600.))).unwrap();
}

fn editor(controller: &TextEditingController, style: &TextStyle) -> EditableText {
    EditableText::new(controller.clone())
        .style(style.clone())
        .text_direction(TextDirection::Ltr)
        // The reference is low-level RenderEditable, whose cursor is 1px.
        // Widget-level EditableText's existing 2px default is unchanged.
        .cursor_width(1.)
}

#[test]
fn an_explicit_scaler_updates_the_intrinsic_width_without_scaling_the_caret_gap() {
    let Some(style) = base_style() else { return };
    let controller = TextEditingController::with_text("Hello World");
    let scene = |scale| {
        Widget::from(editor(&controller, &style).text_scaler(TextScaler::linear(scale)))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(1.)).unwrap();
    frame(&mut tree);
    let render = tree.render_id(root);
    let before = tree.element_bounds(root);
    let layouts = tree.diagnostics().layouts;
    assert_eq!(tree.editable_max_intrinsic_width(root), Some(112.));
    assert_eq!(tree.element_bounds(root), before);
    assert_eq!(tree.diagnostics().layouts, layouts);

    tree.update(root, scene(2.)).unwrap();
    frame(&mut tree);
    assert_eq!(tree.render_id(root), render);
    assert_eq!(tree.editable_max_intrinsic_width(root), Some(222.));
    assert_eq!(tree.editable_intrinsic_height(root, f32::INFINITY), Some(20.));
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 20.);
}

#[test]
fn switching_from_unlimited_lines_to_one_updates_intrinsics_and_the_retained_viewport() {
    let Some(style) = base_style() else { return };
    let controller = TextEditingController::with_text("A\nA\nA\nA\nA");
    let scene = |lines| Widget::from(editor(&controller, &style).max_lines(lines));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(None)).unwrap();
    let render = tree.render_id(root);
    frame(&mut tree);
    assert_eq!(tree.editable_intrinsic_height(root, f32::INFINITY), Some(50.));
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 50.);

    tree.update(root, scene(Some(1))).unwrap();
    frame(&mut tree);
    assert_eq!(tree.render_id(root), render);
    assert_eq!(tree.editable_intrinsic_height(root, f32::INFINITY), Some(10.));
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 10.);
}

#[test]
fn a_forced_strut_controls_both_intrinsic_and_laid_out_height() {
    let Some(style) = base_style() else { return };
    let controller = TextEditingController::with_text("Hello World");
    let scene = |force| {
        let editor = editor(&controller, &style);
        if force {
            editor.strut_style(StrutStyle::new().font_size(100.).force_strut_height(true)).into()
        } else {
            editor.into()
        }
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(false)).unwrap();
    frame(&mut tree);
    let render = tree.render_id(root);
    assert_eq!(tree.editable_intrinsic_height(root, f32::INFINITY), Some(10.));
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 10.);

    tree.update(root, scene(true)).unwrap();
    frame(&mut tree);
    assert_eq!(tree.render_id(root), render);
    assert_eq!(tree.editable_intrinsic_height(root, f32::INFINITY), Some(100.));
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 100.);
    assert_eq!(tree.editable_max_intrinsic_width(root), Some(112.));
}

#[test]
fn an_explicit_scaler_resolves_the_strut_without_following_ambient_scaling() {
    let Some(style) = base_style() else { return };
    let controller = TextEditingController::with_text("Hello World");
    let scene = |explicit| {
        let field = editor(&controller, &style);
        if explicit {
            field
                .text_scaler(TextScaler::linear(2.))
                .strut_style(StrutStyle::new().font_size(50.).force_strut_height(true))
                .into()
        } else {
            field.into()
        }
    };
    let mut tree = WidgetTree::new();
    let mut ambient = tree.environment().clone();
    ambient.text_scale = 3.;
    tree.set_environment(ambient.clone());
    let root = tree.mount(scene(false)).unwrap();
    frame(&mut tree);
    let render = tree.render_id(root);
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 30.);
    assert_eq!(tree.editable_max_intrinsic_width(root), Some(332.));

    tree.update(root, scene(true)).unwrap();
    frame(&mut tree);
    assert_eq!(tree.render_id(root), render);
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 100.);
    assert_eq!(tree.editable_intrinsic_height(root, 200.), Some(100.));
    assert_eq!(tree.editable_max_intrinsic_width(root), Some(222.));

    ambient.text_scale = 4.;
    tree.set_environment(ambient);
    frame(&mut tree);
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 100.);
    assert_eq!(tree.editable_max_intrinsic_width(root), Some(222.));

    tree.update(root, scene(false)).unwrap();
    frame(&mut tree);
    assert_eq!(tree.render_id(root), render);
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 40.);
    assert_eq!(tree.editable_max_intrinsic_width(root), Some(442.));
}

#[test]
fn default_editor_strut_stabilizes_line_height_when_the_glyph_font_falls_back() {
    let Some(style) = base_style() else { return };
    const FALLBACK: &str = "Pinned Editable Inter";
    register_application_font_as(
        FALLBACK,
        include_bytes!("../../../harness/flutter/fonts/Inter-Regular.ttf").to_vec(),
    );
    let style = style.font_size(20.).fallback_families([FALLBACK]);
    let controller = TextEditingController::with_text("AΨ");
    let scene = |disabled| {
        let field = editor(&controller, &style);
        Widget::from(if disabled {
            field.strut_style(StrutStyle::new().font_size(0.))
        } else {
            field
        })
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(false)).unwrap();
    frame(&mut tree);
    let render = tree.render_id(root);
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 20.);
    assert_eq!(tree.editable_intrinsic_height(root, 200.), Some(20.));

    tree.update(root, scene(true)).unwrap();
    frame(&mut tree);
    assert_eq!(tree.render_id(root), render);
    let height = tree.element_bounds(root).unwrap().size.height;
    assert!(height > 20., "fallback line height is {height}");
    assert_eq!(tree.editable_intrinsic_height(root, 200.), Some(height));

    tree.update(root, scene(false)).unwrap();
    frame(&mut tree);
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 20.);

    let before = tree.text_diagnostics();
    for value in ["AAΨ", "ΨA", "AAAAΨ"] {
        controller.set_text(value);
        frame(&mut tree);
        assert_eq!(tree.element_bounds(root).unwrap().size.height, 20.);
    }
    let after = tree.text_diagnostics();
    assert_eq!(after.strut_font_shapes, before.strut_font_shapes);
    assert!(after.strut_font_hits >= before.strut_font_hits + 3);

    register_application_font_as(
        "Late Editor Font",
        include_bytes!("../../../harness/flutter/fonts/Inter-Regular.ttf").to_vec(),
    );
    assert!(tree.has_pending_application_fonts());
    frame(&mut tree);
    assert!(tree.text_diagnostics().strut_font_shapes > after.strut_font_shapes);
    assert_eq!(tree.element_bounds(root).unwrap().size.height, 20.);
}
