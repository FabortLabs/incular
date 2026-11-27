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
