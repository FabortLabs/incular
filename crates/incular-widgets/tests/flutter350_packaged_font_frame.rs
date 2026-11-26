//! A mounted Text reshapes when its package family is registered later.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_text::{FontFamily, TextStyle, register_application_font_as};
use incular_widgets::{Text, internal::WidgetTree};

const INTER: &[u8] = include_bytes!("../../../harness/flutter/fonts/Inter-Regular.ttf");

#[test]
fn a_named_application_font_updates_retained_text_at_the_next_layout() {
    let packaged = FontFamily::packaged("audit_widget", "Body");
    let text = Text::new("ΨΨΨ")
        .text_direction(TextDirection::Ltr)
        .style(TextStyle::new().family(packaged.clone()).size(33.));
    let mut tree = WidgetTree::new();
    let root = tree.mount(text.into()).unwrap();
    let bounds = Constraints::loose(Size::new(800., 600.));
    tree.layout(bounds).unwrap();
    let render = tree.render_id(root).unwrap();
    let before = tree.text_layout(render).unwrap().lines[0].runs[0].font.id();

    let mut distinctive = INTER.to_vec();
    distinctive.extend_from_slice(b"incular widget package fixture");
    register_application_font_as(packaged.name(), distinctive.clone());
    assert!(tree.has_pending_application_fonts());
    tree.layout(bounds).unwrap();
    assert_eq!(tree.render_id(root), Some(render));
    assert!(!tree.has_pending_application_fonts());
    let layout = tree.text_layout(render).unwrap();
    let font = &layout.lines[0].runs[0].font;
    assert_ne!(font.id(), before);
    assert_eq!(font.bytes().as_ref(), distinctive);
}
