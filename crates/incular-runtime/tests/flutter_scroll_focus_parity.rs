//! Flutter's Focus detaches an external FocusNode when an ordinary lazy row
//! is evicted. Merely leaving the visible viewport, while still within the
//! sliver cache, does not detach the node.

use incular_config::Constraints;
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Focus, FocusNode, ListView, ScrollController, SizedBox, Widget, internal::Key,
};
use std::time::{Duration, Instant};

#[test]
fn page_down_releases_focus_node_when_lazy_row_leaves_cache() {
    const ROW_EXTENT: f32 = 50.;
    const CACHE_EXTENT: f32 = 250.;
    let bounds = Constraints::tight(Size::new(800., 600.));
    let controller = ScrollController::new();
    let row_focus = FocusNode::new();
    let first_row_key = Key::from("first row");
    let row_node = row_focus.clone();
    let row_key = first_row_key.clone();
    let list = ListView::builder(30, move |index| {
        let row = SizedBox::expand();
        if index == 0 {
            Widget::from(Focus::new(row).node(row_node.clone()).autofocus(true))
                .with_key(row_key.clone())
        } else {
            row.into()
        }
    })
    .item_extent(ROW_EXTENT)
    .controller(controller.clone());
    let mut runtime = Runtime::new(list.into()).expect("mount list");
    let start = Instant::now();
    let frame = |runtime: &mut Runtime, millis| {
        runtime
            .run_frame_at(bounds, start + Duration::from_millis(millis))
            .expect("render list");
    };
    frame(&mut runtime, 0);
    frame(&mut runtime, 16);
    assert!(row_focus.has_focus(), "first row receives autofocus");
    let original_focus = runtime.focused_element().expect("runtime tracks autofocus");
    assert!(runtime.tree().element_with_key(&first_row_key).is_some());

    let page_down = KeyboardKey::Named(NamedKey::PageDown);
    for event in [
        KeyboardEvent::key_down(page_down.clone(), Code::PageDown),
        KeyboardEvent::key_up(page_down, Code::PageDown),
    ] {
        let _ = runtime.handle_input_at(InputEvent::Key(event), start + Duration::from_millis(16));
    }
    for n in 2..=5 {
        frame(&mut runtime, 16 * n);
    }
    assert!((ROW_EXTENT..CACHE_EXTENT).contains(&controller.offset()));
    assert!(runtime.tree().element_with_key(&first_row_key).is_some());
    assert_eq!(runtime.focused_element(), Some(original_focus));
    assert!(
        row_focus.has_focus(),
        "offscreen but cached row keeps focus"
    );

    for n in 6..=11 {
        frame(&mut runtime, 16 * n);
    }
    assert!((controller.offset() - 480.).abs() < 0.001);
    assert!(runtime.tree().element_with_key(&first_row_key).is_none());
    assert_ne!(runtime.focused_element(), Some(original_focus));
    assert!(
        !row_focus.has_focus(),
        "an externally owned node must not report focus after its lazy row unmounts"
    );
}
