use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_runtime::{Application, FrameStats};
use incular_text::TextEditingController;
use incular_widgets::{EditableText, Text, Widget};

const MESSAGE: &[u8] = br#"{"type":"fontsChange"}"#;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(800.0, 600.0))
}

fn font_change_relayouts_at_the_frame(make_text: impl Fn() -> Widget + 'static) {
    let mut app = Application::new(move |_| make_text()).unwrap();
    let id = app.active_window_ids()[0];
    let frame = |app: &mut Application| -> FrameStats {
        app.run_window_frame_at(id, bounds(), Instant::now())
            .unwrap()
            .unwrap()
            .1
    };
    frame(&mut app);
    assert_eq!(frame(&mut app).laid_out_render_objects, 0);

    assert!(!app.handle_system_message("flutter/other", MESSAGE));
    assert!(!app.handle_system_message("flutter/system", br#"{"type":"other"}"#));
    assert!(app.handle_system_message("flutter/system", MESSAGE));
    assert!(app.frame_requested(id));

    let animation_ran = Rc::new(Cell::new(false));
    let callback_flag = animation_ran.clone();
    incular_animation::scheduler::schedule_frame_callback(move |_| callback_flag.set(true));
    assert!(!animation_ran.get());
    let changed = frame(&mut app);
    assert!(animation_ran.get());
    assert_eq!(changed.updated_elements, 0);
    assert_eq!(changed.rebuilt_elements, 0);
    assert_eq!(changed.laid_out_render_objects, 1);
    assert_eq!(changed.repainted_render_objects, 1);
    assert_eq!(frame(&mut app).laid_out_render_objects, 0);
}

#[test]
fn a_mounted_raw_paragraph_relayouts_on_the_font_notification_frame() {
    font_change_relayouts_at_the_frame(|| {
        Text::new("text widget")
            .text_direction(TextDirection::Ltr)
            .into()
    });
}

#[test]
fn a_mounted_raw_editor_relayouts_on_the_font_notification_frame() {
    font_change_relayouts_at_the_frame(|| {
        EditableText::new(TextEditingController::with_text("text widget"))
            .text_direction(TextDirection::Ltr)
            .into()
    });
}

#[test]
fn a_non_text_tree_does_not_relayout_on_font_change() {
    let mut app = Application::new(|_| Widget::box_(Size::new(30.0, 20.0), Color::BLACK)).unwrap();
    let id = app.active_window_ids()[0];
    let frame = |app: &mut Application| {
        app.run_window_frame_at(id, bounds(), Instant::now())
            .unwrap()
            .unwrap()
            .1
    };
    frame(&mut app);
    assert!(app.handle_system_message("flutter/system", MESSAGE));
    let changed = frame(&mut app);
    assert_eq!(changed.laid_out_render_objects, 0);
    assert_eq!(changed.repainted_render_objects, 0);
}

#[test]
fn a_notice_during_transient_callbacks_waits_for_the_next_frame() {
    let mut runtime =
        incular_runtime::Runtime::new(Text::new("Hello").text_direction(TextDirection::Ltr).into())
            .unwrap();
    runtime.run_frame(bounds()).unwrap();
    let element = runtime.tree().root().unwrap();
    let render = runtime.tree().render_id(element).unwrap();
    let old_metrics = runtime.tree().text_layout(render).unwrap().metrics;

    incular_animation::scheduler::schedule_frame_callback(move |_| {
        incular_text::TextEngine::notify_system_fonts_changed();
    });
    let (_, receiving_frame) = runtime.run_frame(bounds()).unwrap();
    assert_eq!(receiving_frame.laid_out_render_objects, 0);
    assert_eq!(
        runtime.tree().text_layout(render).unwrap().metrics,
        old_metrics
    );
    assert!(runtime.frame_requested());

    let (_, deferred_frame) = runtime.run_frame(bounds()).unwrap();
    assert_eq!(deferred_frame.laid_out_render_objects, 1);
    assert_eq!(runtime.tree().render_id(element), Some(render));
    assert_eq!(
        runtime.tree().text_layout(render).unwrap().metrics,
        old_metrics
    );
}

#[test]
fn banner_is_paint_dirty_before_the_next_font_update_frame() {
    use incular_runtime::Runtime;
    use incular_widgets::{Banner, BannerLocation};

    let banner = Banner::new("message", BannerLocation::TopStart)
        .text_direction(TextDirection::Ltr)
        .layout_direction(TextDirection::Ltr)
        .child(Text::new("dependent paragraph").text_direction(TextDirection::Ltr));
    let mut runtime = Runtime::new(banner.into()).unwrap();
    runtime.run_frame(bounds()).unwrap();
    let banner_element = runtime.tree().root().unwrap();
    let banner_id = runtime.tree().render_id(banner_element).unwrap();
    let paragraph = runtime.tree().children(banner_element).unwrap()[0];
    let paragraph_id = runtime.tree().render_id(paragraph).unwrap();
    let metrics = runtime.tree().text_layout(paragraph_id).unwrap().metrics;
    assert_eq!(runtime.tree().render_needs_paint(banner_id), Some(false));

    let app = Application::from_runtime(runtime, |_| {});
    assert!(app.handle_system_message("flutter/system", MESSAGE));
    let mut runtime = app.into_runtime();
    assert_eq!(runtime.tree().render_needs_paint(banner_id), Some(true));
    assert_eq!(runtime.tree().render_needs_layout(banner_id), Some(false));
    assert_eq!(
        runtime.tree().render_needs_layout(paragraph_id),
        Some(false)
    );
    assert_eq!(runtime.tree().render_needs_paint(paragraph_id), Some(false));
    assert_eq!(
        runtime.tree().text_layout(paragraph_id).unwrap().metrics,
        metrics
    );

    let (_, frame) = runtime.run_frame(bounds()).unwrap();
    assert!(frame.repainted_render_objects > 0);
    assert_eq!(runtime.tree().render_needs_paint(banner_id), Some(false));
    assert_eq!(runtime.tree().render_id(banner_element), Some(banner_id));
}
