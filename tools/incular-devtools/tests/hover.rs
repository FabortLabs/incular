use incular_devtools_protocol::{DevWidgetId, DevWindowId, RequestMethod};
use incular_devtools_ui::inspector::{ActiveHighlight, ConnectionState, InspectorModel, ModelCell};
use std::sync::Arc;

fn id(index: u64) -> DevWidgetId {
    DevWidgetId::new(index, 0)
}

fn window() -> DevWindowId {
    DevWindowId::new(1, 0)
}

fn connected() -> InspectorModel {
    let mut model = InspectorModel::default();
    model.transition(ConnectionState::Connected, None, None);
    model.activate_window(window());
    model
}

fn node(id: DevWidgetId) -> RequestMethod {
    RequestMethod::HighlightNode {
        window: window(),
        id: Some(id),
    }
}

fn clear() -> RequestMethod {
    RequestMethod::HighlightNode {
        window: window(),
        id: None,
    }
}

#[test]
fn hover_highlights_only_while_the_pointer_is_over_the_row() {
    let mut model = connected();
    assert_eq!(model.begin_hover(id(3)), vec![node(id(3))]);
    assert_eq!(
        model.active_highlight(),
        Some(ActiveHighlight::Node(window(), id(3)))
    );
    assert_eq!(model.end_hover(id(3)), Some(clear()));
    assert_eq!(model.active_highlight(), None);
    // Nothing is left to clear afterwards.
    assert_eq!(model.clear_highlight(), None);
}

#[test]
fn hovering_the_same_row_twice_sends_one_request() {
    let mut model = connected();
    assert_eq!(model.begin_hover(id(3)).len(), 1);
    assert!(model.begin_hover(id(3)).is_empty());
}

#[test]
fn late_exit_from_the_previous_row_keeps_the_new_highlight() {
    let mut model = connected();
    model.begin_hover(id(2));
    // The next row is entered before the old row reports its exit.
    assert_eq!(model.begin_hover(id(3)), vec![node(id(3))]);
    assert_eq!(model.end_hover(id(2)), None);
    assert_eq!(
        model.active_highlight(),
        Some(ActiveHighlight::Node(window(), id(3)))
    );
}

#[test]
fn disabling_hover_highlight_clears_and_suppresses_requests() {
    let mut model = connected();
    model.begin_hover(id(3));
    assert_eq!(model.set_hover_highlight(false), Some(clear()));
    assert!(!model.hover_highlight_enabled());
    assert!(model.begin_hover(id(4)).is_empty());
    assert_eq!(model.end_hover(id(4)), None);
    assert_eq!(model.set_hover_highlight(true), None);
    assert_eq!(model.begin_hover(id(4)), vec![node(id(4))]);
}

#[test]
fn selection_alone_never_creates_an_overlay() {
    let mut model = connected();
    // Selecting a row does not go through the hover API and records nothing.
    model.reveal(id(1));
    assert_eq!(model.active_highlight(), None);
    assert_eq!(model.clear_highlight(), None);
}

#[test]
fn rect_hover_replaces_node_hover_and_clears_the_right_overlay() {
    let mut model = connected();
    model.begin_hover(id(3));
    let rect = [1., 2., 30., 40.];
    assert_eq!(
        model.begin_hover_rect(7, Some(rect)),
        vec![
            clear(),
            RequestMethod::HighlightRect {
                window: window(),
                rect: Some(rect)
            }
        ]
    );
    assert_eq!(
        model.end_hover_rect(7),
        Some(RequestMethod::HighlightRect {
            window: window(),
            rect: None
        })
    );
    assert_eq!(model.end_hover_rect(7), None);
}

#[test]
fn rows_without_geometry_show_nothing() {
    let mut model = connected();
    assert!(model.begin_hover_rect(1, None).is_empty());
    assert_eq!(model.active_highlight(), None);
}

#[test]
fn picker_clears_the_overlay_and_always_stops() {
    let mut model = connected();
    model.begin_hover(id(3));
    let requests = model.begin_pick();
    assert_eq!(
        requests,
        vec![
            clear(),
            RequestMethod::StartInspectMode { window: window() }
        ]
    );
    assert!(model.select_mode_active());
    // No hover overlay while the picker owns the pointer.
    assert!(model.begin_hover(id(4)).is_empty());
    assert_eq!(
        model.end_pick(),
        Some(RequestMethod::StopInspectMode { window: window() })
    );
    assert!(!model.select_mode_active());
    assert_eq!(model.end_pick(), None);
}

#[test]
fn disconnecting_drops_picker_and_overlay_state() {
    let mut model = connected();
    model.begin_hover(id(3));
    model.begin_pick();
    model.transition(ConnectionState::Disconnected, Some("gone".into()), None);
    assert!(!model.select_mode_active());
    assert_eq!(model.active_highlight(), None);
    assert_eq!(model.end_pick(), None);
}

#[test]
fn a_panic_while_the_model_is_locked_does_not_wedge_later_locks() {
    let shared = Arc::new(ModelCell::new(InspectorModel::default()));
    let poisoner = Arc::clone(&shared);
    let result = std::thread::spawn(move || {
        let _guard = poisoner.lock().expect("lock");
        panic!("simulated view bug while holding the model");
    })
    .join();
    assert!(result.is_err());
    let mut model = shared.lock().expect("recovered lock");
    model.transition(ConnectionState::Connected, None, None);
    assert!(model.connected_now());
}
