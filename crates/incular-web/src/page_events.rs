//! Page-level event listeners that run before Winit's.
//!
//! Winit listens on the canvas and cancels every key press. Capture
//! listeners on the window see each event first and:
//!
//! - record its modifiers, because Winit reports `ModifiersChanged` after
//!   the key it came with;
//! - leave browser shortcuts (reload, zoom, developer tools, address bar) to
//!   the browser instead of letting Winit cancel them;
//! - hold paste chords for the [page clipboard](crate::clipboard).
//!
//! It also refuses file drops on canvases, which the browser would otherwise
//! open in place of the application.

use std::cell::Cell;

use wasm_bindgen::{JsCast as _, closure::Closure};
use winit::keyboard::ModifiersState;

thread_local! {
    /// Modifiers of the key event Winit is about to handle.
    static MODIFIERS: Cell<Option<ModifiersState>> = const { Cell::new(None) };
}

/// Installs the listeners once per page.
pub(crate) fn install() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let keydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(key_down);
    let keyup = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|event| {
        record_modifiers(&event);
    });
    for (kind, listener) in [("keydown", &keydown), ("keyup", &keyup)] {
        let _ = window.add_event_listener_with_callback_and_bool(
            kind,
            listener.as_ref().unchecked_ref(),
            true,
        );
    }
    // The browser opens a dropped file by navigating away from the page.
    // Incular reports external drops unsupported in browsers, so a drop on
    // a canvas does nothing.
    let refuse_drop = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        if event
            .target()
            .is_some_and(|target| target.dyn_ref::<web_sys::HtmlCanvasElement>().is_some())
        {
            event.prevent_default();
        }
    });
    for kind in ["dragover", "drop"] {
        let _ = window.add_event_listener_with_callback_and_bool(
            kind,
            refuse_drop.as_ref().unchecked_ref(),
            true,
        );
    }
    // The listeners live as long as the page.
    keydown.forget();
    keyup.forget();
    refuse_drop.forget();
}

/// Modifiers of the key event being delivered. Without this, Ctrl+A is
/// handled as a bare "a" whenever Ctrl went down before the page saw it.
pub(crate) fn current_modifiers() -> Option<ModifiersState> {
    MODIFIERS.get()
}

fn key_down(event: web_sys::KeyboardEvent) {
    record_modifiers(&event);
    // Replayed paste chords are untrusted and must reach Winit.
    if !event.is_trusted() {
        return;
    }
    // While a text field is focused, the editing element receives the keys,
    // and a focused semantic element while a screen reader is in use.
    let Some(target) = event.target().filter(|target| {
        target.dyn_ref::<web_sys::HtmlCanvasElement>().is_some()
            || crate::text_input::is_editor(target)
            || crate::semantics::is_semantics_element(target)
    }) else {
        return;
    };
    if is_browser_shortcut(&event) {
        // Winit would cancel it; the application never sees it.
        event.stop_immediate_propagation();
        return;
    }
    crate::clipboard::hold_paste_chord(&event, target);
}

fn record_modifiers(event: &web_sys::KeyboardEvent) {
    let mut modifiers = ModifiersState::empty();
    modifiers.set(ModifiersState::SHIFT, event.shift_key());
    modifiers.set(ModifiersState::CONTROL, event.ctrl_key());
    modifiers.set(ModifiersState::ALT, event.alt_key());
    modifiers.set(ModifiersState::SUPER, event.meta_key());
    MODIFIERS.set(Some(modifiers));
}

/// Shortcuts users expect the browser to keep. Chords browsers never let a
/// page cancel (new tab, close tab) are left to the application, which
/// still sees them.
fn is_browser_shortcut(event: &web_sys::KeyboardEvent) -> bool {
    let command = event.ctrl_key() || event.meta_key();
    // Developer tools: Ctrl+Shift+I/J/C, or Cmd+Option+I/J/C. Option changes
    // the reported key on macOS, so these match the physical key.
    let developer_tools = matches!(event.code().as_str(), "KeyI" | "KeyJ" | "KeyC")
        && command
        && (event.shift_key() || (event.meta_key() && event.alt_key()));
    developer_tools
        || match event.key().as_str() {
            "F5" | "F11" | "F12" => true,
            // Reload, hard reload, address bar and page zoom.
            "r" | "R" | "l" | "L" | "+" | "=" | "-" | "_" | "0" => command && !event.alt_key(),
            _ => false,
        }
}
