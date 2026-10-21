//! Page clipboard for Incular windows.
//!
//! The Incular clipboard is synchronous; the browser's is asynchronous and
//! only readable from a `paste` event. Plain text is therefore kept in a page
//! buffer:
//!
//! - Writes update the buffer and are mirrored to the system clipboard with
//!   `navigator.clipboard.writeText` (copying happens inside the key event
//!   that requested it, which the browser requires).
//! - Ctrl/Cmd+V on an Incular canvas is held back from Winit, which would
//!   cancel the key press and with it the browser's `paste` event. The
//!   `paste` event's text refreshes the buffer and the chord is then replayed
//!   to Winit, so the application reads the system clipboard's text.
//!
//! Other formats are reported unsupported.

use std::cell::RefCell;

use incular_platform::{
    CapabilitySupport, Clipboard, ClipboardCapabilities, ClipboardError, ClipboardWriteReport,
    DataTransfer, TransferFormat, TransferFormatCapabilities, TransferReadRequest,
};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};

/// How long a held-back paste chord waits for the browser's `paste` event
/// before it is replayed with the current buffer.
const PASTE_FALLBACK_MS: i32 = 100;

thread_local! {
    static TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    static PENDING_PASTE: RefCell<Option<Chord>> = const { RefCell::new(None) };
    /// Keeps refused system writes out of the console as unhandled rejections.
    static IGNORE_REJECTION: Closure<dyn FnMut(JsValue)> = Closure::new(|_| {});
}

/// A held-back Ctrl/Cmd+V key press and the canvas it was aimed at.
struct Chord {
    target: web_sys::EventTarget,
    key: String,
    code: String,
    ctrl: bool,
    meta: bool,
    shift: bool,
}

pub(crate) struct PageClipboard;

impl Clipboard for PageClipboard {
    fn capabilities(&self) -> ClipboardCapabilities {
        let plain_text = TransferFormatCapabilities {
            plain_text: CapabilitySupport::Supported,
            ..TransferFormatCapabilities::all(CapabilitySupport::Unsupported)
        };
        ClipboardCapabilities {
            read: plain_text,
            write: plain_text,
            lazy_write: CapabilitySupport::Unsupported,
        }
    }

    fn read(&mut self, request: TransferReadRequest) -> Result<DataTransfer, ClipboardError> {
        let text = TEXT
            .with_borrow(Clone::clone)
            .ok_or(ClipboardError::Unavailable)?;
        let filtered = DataTransfer::plain_text(text).filtered(&request);
        if filtered.is_empty() {
            Err(ClipboardError::Unavailable)
        } else {
            Ok(filtered)
        }
    }

    fn write(&mut self, transfer: DataTransfer) -> Result<ClipboardWriteReport, ClipboardError> {
        let Some(text) = transfer.plain_text_value()? else {
            return Err(transfer.formats().into_iter().next().map_or(
                ClipboardError::Unavailable,
                ClipboardError::UnsupportedFormat,
            ));
        };
        write_system_text(&text);
        TEXT.set(Some(text.to_string()));
        let mut report = ClipboardWriteReport::all_written(vec![TransferFormat::PlainText]);
        report.skipped = transfer
            .formats()
            .into_iter()
            .filter(|format| *format != TransferFormat::PlainText)
            .collect();
        Ok(report)
    }
}

/// Mirrors text to the system clipboard. Insecure origins have no
/// `navigator.clipboard`, and the browser may refuse the write; the page
/// buffer still holds the text either way.
fn write_system_text(text: &str) {
    let Some(navigator) = web_sys::window().map(|window| window.navigator()) else {
        return;
    };
    let available = js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard"))
        .is_ok_and(|clipboard| !clipboard.is_undefined() && !clipboard.is_null());
    if !available {
        return;
    }
    let promise = navigator.clipboard().write_text(text);
    IGNORE_REJECTION.with(|ignore| {
        let _ = promise.catch(ignore);
    });
}

/// Installs the page-level `paste` listener once per page.
pub(crate) fn install() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let paste = Closure::<dyn FnMut(web_sys::ClipboardEvent)>::new(take_paste);
    let _ = window.add_event_listener_with_callback_and_bool(
        "paste",
        paste.as_ref().unchecked_ref(),
        true,
    );
    // The listener lives as long as the page.
    paste.forget();
}

/// Holds a trusted Ctrl/Cmd+V aimed at a canvas back from Winit until the
/// browser's `paste` event has refreshed the buffer. Returns whether the
/// event was held.
pub(crate) fn hold_paste_chord(
    event: &web_sys::KeyboardEvent,
    target: web_sys::EventTarget,
) -> bool {
    if event.alt_key()
        || !(event.ctrl_key() || event.meta_key())
        || !event.key().eq_ignore_ascii_case("v")
    {
        return false;
    }
    // Not `preventDefault`: the default action is the `paste` event.
    event.stop_immediate_propagation();
    PENDING_PASTE.set(Some(Chord {
        target,
        key: event.key(),
        code: event.code(),
        ctrl: event.ctrl_key(),
        meta: event.meta_key(),
        shift: event.shift_key(),
    }));
    if let Some(window) = web_sys::window() {
        let fallback = Closure::once_into_js(replay_pending_paste);
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            fallback.unchecked_ref(),
            PASTE_FALLBACK_MS,
        );
    }
    true
}

fn take_paste(event: web_sys::ClipboardEvent) {
    if PENDING_PASTE.with_borrow(Option::is_none) {
        return;
    }
    event.prevent_default();
    if let Some(text) = event
        .clipboard_data()
        .and_then(|data| data.get_data("text/plain").ok())
        .filter(|text| !text.is_empty())
    {
        TEXT.set(Some(text));
    }
    replay_pending_paste();
}

fn replay_pending_paste() {
    let Some(chord) = PENDING_PASTE.take() else {
        return;
    };
    for kind in ["keydown", "keyup"] {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(&chord.key);
        init.set_code(&chord.code);
        init.set_ctrl_key(chord.ctrl);
        init.set_meta_key(chord.meta);
        init.set_shift_key(chord.shift);
        init.set_bubbles(true);
        init.set_cancelable(true);
        if let Ok(event) = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(kind, &init) {
            let _ = chord.target.dispatch_event(&event);
        }
    }
}
