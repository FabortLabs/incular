//! Pointer-driven target overlays and the widget picker.
//!
//! The target outlines a widget only while the pointer is over its row in
//! DevTools (like a browser's element inspector). Selecting a row never
//! leaves an overlay behind, and every path that could strand one - leaving
//! the row, switching views, scrolling the row away, disconnecting - clears
//! it. All state transitions live in [`InspectorModel`]; these helpers only
//! lock, apply and send.
//!
//! [`InspectorModel`]: crate::inspector::InspectorModel

use super::shared::refresh;
use crate::{inspector::Shared, transport::ClientBridge};
use incular::prelude::Signal;
use incular_devtools_protocol::{DevWidgetId, RequestMethod};

fn send_all(bridge: &ClientBridge, requests: impl IntoIterator<Item = RequestMethod>) {
    for request in requests {
        bridge.send_or_report(request);
    }
}

/// The pointer entered the row of widget `id`.
pub(crate) fn enter_node(shared: &Shared, bridge: &ClientBridge, id: DevWidgetId) {
    let requests = shared
        .lock()
        .map(|mut state| state.begin_hover(id))
        .unwrap_or_default();
    send_all(bridge, requests);
}

/// The pointer left the row of widget `id`.
pub(crate) fn leave_node(shared: &Shared, bridge: &ClientBridge, id: DevWidgetId) {
    let request = shared.lock().ok().and_then(|mut state| state.end_hover(id));
    send_all(bridge, request);
}

/// The pointer entered a semantics or layer row with these window bounds.
pub(crate) fn enter_rect(shared: &Shared, bridge: &ClientBridge, row: u64, rect: Option<[f32; 4]>) {
    let requests = shared
        .lock()
        .map(|mut state| state.begin_hover_rect(row, rect))
        .unwrap_or_default();
    send_all(bridge, requests);
}

/// The pointer left a semantics or layer row.
pub(crate) fn leave_rect(shared: &Shared, bridge: &ClientBridge, row: u64) {
    let request = shared
        .lock()
        .ok()
        .and_then(|mut state| state.end_hover_rect(row));
    send_all(bridge, request);
}

/// Clears the target overlay unconditionally (view switch, scroll, toggle).
pub(crate) fn clear(shared: &Shared, bridge: &ClientBridge) {
    let request = shared.lock().ok().and_then(|mut state| {
        state.hovered = None;
        state.hovered_row = None;
        state.clear_highlight()
    });
    send_all(bridge, request);
}

/// Toggles "Highlight on hover".
pub(crate) fn set_enabled(shared: &Shared, bridge: &ClientBridge, tick: &Signal<u64>, on: bool) {
    let request = shared
        .lock()
        .ok()
        .and_then(|mut state| state.set_hover_highlight(on));
    send_all(bridge, request);
    refresh(tick);
}

/// Starts (`Some(true)`), stops (`Some(false)`) or toggles (`None`) the
/// target picker. Returns whether a window was available.
pub(crate) fn pick(
    shared: &Shared,
    bridge: &ClientBridge,
    tick: &Signal<u64>,
    enabled: Option<bool>,
) -> bool {
    let requests = shared.lock().ok().map(|mut state| {
        let start = enabled.unwrap_or(!state.select_mode);
        if start {
            if state.select_mode {
                Vec::new()
            } else {
                state.begin_pick()
            }
        } else {
            state.end_pick().into_iter().collect()
        }
    });
    let handled = requests
        .as_ref()
        .is_some_and(|requests| !requests.is_empty());
    send_all(bridge, requests.unwrap_or_default());
    refresh(tick);
    handled
}

/// Cancels the picker if it is active; safe to call on every navigation.
pub(crate) fn cancel_pick(shared: &Shared, bridge: &ClientBridge) {
    let request = shared.lock().ok().and_then(|mut state| state.end_pick());
    send_all(bridge, request);
}
