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
