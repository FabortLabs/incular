use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_text::{TextEditingController, TextEngine};
use incular_widgets::{EditableText, Text, internal::WidgetTree};

#[test]
fn system_font_notice_keeps_text_layout_clean_until_frame_boundary() {
    for widget in [
        Text::new("paragraph")
            .text_direction(TextDirection::Ltr)
            .into(),
        EditableText::new(TextEditingController::with_text("editor"))
            .text_direction(TextDirection::Ltr)
            .into(),
    ] {
        let mut tree = WidgetTree::new();
        let id = tree.mount(widget).unwrap();
        let render = tree.render_id(id).unwrap();
        tree.layout(Constraints::tight(Size::new(300.0, 80.0)))
            .unwrap();
        let _ = tree.paint();
        assert_eq!(tree.render_needs_layout(render), Some(false));
        TextEngine::notify_system_fonts_changed();
        assert!(tree.has_pending_system_fonts());
        assert_eq!(tree.render_needs_layout(render), Some(false));
        tree.apply_system_font_change();
        assert_eq!(tree.render_needs_layout(render), Some(true));
        assert!(!tree.has_pending_system_fonts());
    }
}
