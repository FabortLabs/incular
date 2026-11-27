use std::sync::Arc;

use incular_text::{TextAlign, TextEngine, TextStyle, register_application_font_as};

const INTER: &[u8] = include_bytes!("../../../harness/flutter/fonts/Inter-Regular.ttf");

#[test]
fn queued_notifications_rebuild_once_and_preserve_local_and_application_faces() {
    let mut engine = TextEngine::new();
    let mut local_bytes = INTER.to_vec();
    local_bytes.extend_from_slice(b"local font");
    let mut app_bytes = INTER.to_vec();
    app_bytes.extend_from_slice(b"application font");
    assert!(engine.register_font_bytes_as("test local alias", local_bytes.clone()));
    register_application_font_as("test application alias", app_bytes.clone());

    let local = TextStyle::new().font_family("test local alias");
    let application = TextStyle::new().font_family("test application alias");
    let layout = |engine: &mut TextEngine, style: &TextStyle| {
        engine.layout("typed text", style, None, TextAlign::Start)
    };
    let old_local = layout(&mut engine, &local);
    let old_app = layout(&mut engine, &application);
    let original_generation = engine.font_database_generation();
    TextEngine::notify_system_fonts_changed();
    TextEngine::notify_system_fonts_changed();

    assert!(engine.has_pending_system_fonts());
    assert_eq!(engine.font_database_generation(), original_generation);
    assert!(Arc::ptr_eq(&old_local, &layout(&mut engine, &local)));

    assert!(engine.sync_system_fonts());
    assert!(!engine.has_pending_system_fonts());
    assert!(!engine.sync_system_fonts());
    assert_eq!(engine.font_database_generation(), original_generation + 1);
    for (old, style, bytes) in [
        (old_local, local, local_bytes),
        (old_app, application, app_bytes),
    ] {
        let new = layout(&mut engine, &style);
        assert!(!Arc::ptr_eq(&old, &new));
        assert_eq!(old.metrics, new.metrics);
        assert_eq!(old.lines[0].runs[0].font.bytes().as_ref(), bytes);
        assert_eq!(new.lines[0].runs[0].font.bytes().as_ref(), bytes);
    }
}

#[test]
fn engines_without_system_discovery_ignore_a_host_notice() {
    let mut engine = TextEngine::with_system_fonts(false);
    let original_generation = engine.font_database_generation();
    TextEngine::notify_system_fonts_changed();
    assert!(!engine.has_pending_system_fonts());
    assert!(!engine.sync_system_fonts());
    assert_eq!(engine.font_database_generation(), original_generation);
}
