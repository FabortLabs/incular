use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Rect, Size, time::Instant};
use incular_rendering::{DisplayList, PaintCommand};
use incular_text::{
    TextAffinity, TextEditingController, TextSelection, TextStyle, register_application_font_as,
};
use incular_widgets::{EditableText, Widget, internal::WidgetTree};

const FAMILY: &str = "Fixed Viewport Pinned Ahem";

fn style() -> Option<TextStyle> {
    let path = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../.tmp/flutter/packages/flutter_tools/static/Ahem.ttf")
        });
    if !path.is_file() {
        eprintln!("Pinned Flutter Ahem missing; fixed viewport comparison not run");
        return None;
    }
    register_application_font_as(FAMILY, std::fs::read(path).unwrap());
    Some(
        TextStyle::new()
            .font_family(FAMILY)
            .font_size(10.)
            .height(Some(1.)),
    )
}

fn paint(tree: &mut WidgetTree) -> DisplayList {
    tree.layout(Constraints::loose(Size::new(500., 500.)))
        .unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    tree.paint()
}

#[test]
fn fixed_horizontal_offset_clips_fitting_text_and_does_not_relayout_on_change() {
    let Some(style) = style() else { return };
    let controller = TextEditingController::with_text("A");
    controller.set_selection(TextSelection::collapsed(0));
    let field = |offset| {
        Widget::from(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .style(style.clone())
                .fixed_scroll_offset(offset),
        )
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(field(None)).unwrap();
    let render = tree.render_id(root).unwrap();
    assert!(
        paint(&mut tree)
            .commands()
            .iter()
            .all(|command| { !matches!(command, PaintCommand::PushClip { .. }) })
    );
    let initial_layouts = tree.diagnostics().layouts;
    let initial_caret = tree
        .editable_local_caret_rect(root, 0, TextAffinity::Downstream)
        .unwrap();
    assert_eq!(tree.editable_max_scroll_extent(root), Some(0.));

    tree.update(root, field(Some(10.))).unwrap();
    assert_eq!(tree.render_id(root), Some(render));
    assert_eq!(tree.render_needs_layout(render), Some(false));
    let clip = Rect::from_origin_size(Offset::ZERO, Size::new(500., 10.));
    assert_eq!(tree.editable_approximate_paint_clip(root), Some(clip));
    let list = paint(&mut tree);
    assert_eq!(tree.diagnostics().layouts, initial_layouts);
    assert!(
        list.commands()
            .iter()
            .any(|command| { matches!(command, PaintCommand::PushClip { rect } if *rect == clip) })
    );
    assert_eq!(
        tree.text_field_scroll_offset(root),
        Some(Offset::new(10., 0.))
    );
    let caret = tree
        .editable_local_caret_rect(root, 0, TextAffinity::Downstream)
        .unwrap();
    assert_eq!(caret.origin, initial_caret.origin - Offset::new(10., 0.));
    assert!(!tree.text_field_scroll_page(root, false));
    assert_eq!(
        tree.text_field_scroll_offset(root),
        Some(Offset::new(10., 0.))
    );

    tree.update(root, field(None)).unwrap();
    let _ = paint(&mut tree);
    assert_eq!(tree.text_field_scroll_offset(root), Some(Offset::ZERO));
    assert_eq!(tree.editable_approximate_paint_clip(root), None);
    assert_eq!(tree.diagnostics().layouts, initial_layouts);
}

#[test]
fn fixed_multiline_offset_is_vertical_for_hit_testing_and_ignores_caret_reveal() {
    let Some(style) = style() else { return };
    let controller = TextEditingController::with_text("A\nB");
    controller.set_selection(TextSelection::collapsed(0));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            EditableText::new(controller)
                .text_direction(TextDirection::Ltr)
                .style(style)
                .max_lines(None)
                .fixed_scroll_offset(10.)
                .into(),
        )
        .unwrap();
    let _ = paint(&mut tree);
    assert_eq!(
        tree.text_field_scroll_offset(root),
        Some(Offset::new(0., 10.))
    );
    assert_eq!(
        tree.text_field_position_at(root, Offset::new(0., 1.))
            .unwrap()
            .0,
        2
    );
    assert!(!tree.text_field_bring_position_into_view(root, 0, TextAffinity::Downstream));
    assert_eq!(
        tree.text_field_scroll_offset(root),
        Some(Offset::new(0., 10.))
    );
}
