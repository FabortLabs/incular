//! Page clipboard for Incular windows.
//!
//! The Incular clipboard is synchronous; the browser's is asynchronous and
//! only readable from a `paste` event. Clipboard contents are therefore kept
//! in a page buffer holding plain text, HTML and an RGBA image:
//!
//! - Writes update the buffer and are mirrored to the system clipboard with
//!   `navigator.clipboard` (copying happens inside the key event that
//!   requested it, which the browser requires). Images are encoded as PNG.
//! - Ctrl/Cmd+V on an Incular canvas is held back from Winit, which would
//!   cancel the key press and with it the browser's `paste` event. The
//!   `paste` event's text, HTML and image refresh the buffer and the chord is
//!   then replayed to Winit, so the application reads the system clipboard.
//!   A pasted image is decoded first, which delays the replay until it is.
//!
//! URI lists, files and custom formats are reported unsupported.

use std::cell::RefCell;

use incular_platform::{
    CapabilitySupport, Clipboard, ClipboardCapabilities, ClipboardError, ClipboardWriteReport,
    DataTransfer, TransferData, TransferFormat, TransferFormatCapabilities, TransferImage,
    TransferItem, TransferReadRequest, TransferRepresentation,
};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};

/// How long a held-back paste chord waits for the browser's `paste` event
/// before it is replayed with the current buffer.
const PASTE_FALLBACK_MS: i32 = 100;

/// The formats the page clipboard keeps and mirrors.
const FORMATS: [TransferFormat; 3] = [
    TransferFormat::PlainText,
    TransferFormat::Html,
    TransferFormat::Rgba8Image,
];

thread_local! {
    static BUFFER: RefCell<Option<DataTransfer>> = const { RefCell::new(None) };
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
        let formats = TransferFormatCapabilities {
            plain_text: CapabilitySupport::Supported,
            html: CapabilitySupport::Supported,
            rgba8_image: CapabilitySupport::Supported,
            ..TransferFormatCapabilities::all(CapabilitySupport::Unsupported)
        };
        ClipboardCapabilities {
            read: formats,
            write: formats,
            lazy_write: CapabilitySupport::Unsupported,
        }
    }

    fn read(&mut self, request: TransferReadRequest) -> Result<DataTransfer, ClipboardError> {
        let buffer = BUFFER
            .with_borrow(Clone::clone)
            .ok_or(ClipboardError::Unavailable)?;
        let filtered = buffer.filtered(&request);
        if filtered.is_empty() {
            Err(ClipboardError::Unavailable)
        } else {
            Ok(filtered)
        }
    }

    fn write(&mut self, transfer: DataTransfer) -> Result<ClipboardWriteReport, ClipboardError> {
        let contents = Contents::from_transfer(&transfer)?;
        let Some(kept) = contents.transfer() else {
            return Err(transfer.formats().into_iter().next().map_or(
                ClipboardError::Unavailable,
                ClipboardError::UnsupportedFormat,
            ));
        };
        contents.write_system();
        let written = kept.formats();
        BUFFER.set(Some(kept));
        let mut report = ClipboardWriteReport::all_written(written.clone());
        report.skipped = transfer
            .formats()
            .into_iter()
            .filter(|format| !written.contains(format))
            .collect();
        Ok(report)
    }
}

/// The representations the page clipboard keeps.
#[derive(Default)]
struct Contents {
    text: Option<String>,
    html: Option<String>,
    image: Option<TransferImage>,
}

impl Contents {
    fn from_transfer(transfer: &DataTransfer) -> Result<Self, ClipboardError> {
        let mut contents = Self::default();
        for format in FORMATS {
            match transfer.materialize(&format)? {
                Some(TransferData::Text(text)) if format == TransferFormat::PlainText => {
                    contents.text = Some(text.to_string());
                }
                Some(TransferData::Text(html)) => contents.html = Some(html.to_string()),
                Some(TransferData::Rgba8Image(image)) => contents.image = Some(image),
                _ => {}
            }
        }
        Ok(contents)
    }

    fn transfer(&self) -> Option<DataTransfer> {
        let representations = self
            .text
            .clone()
            .map(TransferRepresentation::plain_text)
            .into_iter()
            .chain(self.html.clone().map(TransferRepresentation::html))
            .chain(self.image.clone().map(TransferRepresentation::rgba8_image))
            .collect::<Vec<_>>();
        if representations.is_empty() {
            return None;
        }
        TransferItem::new(representations)
            .ok()
            .map(DataTransfer::single)
    }

    /// Mirrors the contents to the system clipboard. Insecure origins have no
    /// `navigator.clipboard`, and the browser may refuse the write; the page
    /// buffer still holds the contents either way.
    fn write_system(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let navigator = window.navigator();
        let available = js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard"))
            .is_ok_and(|clipboard| !clipboard.is_undefined() && !clipboard.is_null());
        if !available {
            return;
        }
        let clipboard = navigator.clipboard();
        let promise = if self.html.is_none() && self.image.is_none() {
            let Some(text) = &self.text else {
                return;
            };
            clipboard.write_text(text)
        } else {
            match self.clipboard_item(&window) {
                Some(item) => clipboard.write(&js_sys::Array::of1(&item)),
                // Browsers without `ClipboardItem` still take the text.
                None => match &self.text {
                    Some(text) => clipboard.write_text(text),
                    None => return,
                },
            }
        };
        IGNORE_REJECTION.with(|ignore| {
            let _ = promise.catch(ignore);
        });
    }

    fn clipboard_item(&self, window: &web_sys::Window) -> Option<JsValue> {
        let constructor = js_sys::Reflect::get(window, &JsValue::from_str("ClipboardItem"))
            .ok()?
            .dyn_into::<js_sys::Function>()
            .ok()?;
        let record = js_sys::Object::new();
        for (media_type, value) in [("text/plain", &self.text), ("text/html", &self.html)] {
            if let Some(value) = value {
                js_sys::Reflect::set(
                    &record,
                    &JsValue::from_str(media_type),
                    &text_blob(value, media_type)?,
                )
                .ok()?;
            }
        }
        if let Some(image) = &self.image {
            js_sys::Reflect::set(&record, &JsValue::from_str("image/png"), &png_blob(image)?)
                .ok()?;
        }
        js_sys::Reflect::construct(&constructor, &js_sys::Array::of1(&record)).ok()
    }
}

fn text_blob(text: &str, media_type: &str) -> Option<JsValue> {
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(media_type);
    web_sys::Blob::new_with_str_sequence_and_options(
        &js_sys::Array::of1(&JsValue::from_str(text)),
        &options,
    )
    .ok()
    .map(Into::into)
}

/// A promise of `image` encoded as a PNG blob, which `ClipboardItem` takes in
/// place of the blob.
fn png_blob(image: &TransferImage) -> Option<JsValue> {
    let canvas = scratch_canvas(image.width(), image.height())?;
    let context = context_2d(&canvas)?;
    let data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(image.rgba()),
        image.width(),
        image.height(),
    )
    .ok()?;
    context.put_image_data(&data, 0.0, 0.0).ok()?;
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = canvas.to_blob_with_type(&resolve, "image/png");
    });
    Some(promise.into())
}

fn scratch_canvas(width: u32, height: u32) -> Option<web_sys::HtmlCanvasElement> {
    let canvas = web_sys::window()?
        .document()?
        .create_element("canvas")
        .ok()?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .ok()?;
    canvas.set_width(width);
    canvas.set_height(height);
    Some(canvas)
}

fn context_2d(canvas: &web_sys::HtmlCanvasElement) -> Option<web_sys::CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .ok()
}

/// Decodes an image file to unpremultiplied RGBA.
async fn decode_image(file: web_sys::Blob) -> Option<TransferImage> {
    let window = web_sys::window()?;
    let bitmap =
        wasm_bindgen_futures::JsFuture::from(window.create_image_bitmap_with_blob(&file).ok()?)
            .await
            .ok()?
            .dyn_into::<web_sys::ImageBitmap>()
            .ok()?;
    let (width, height) = (bitmap.width(), bitmap.height());
    let canvas = scratch_canvas(width, height)?;
    let context = context_2d(&canvas)?;
    context
        .draw_image_with_image_bitmap(&bitmap, 0.0, 0.0)
        .ok()?;
    bitmap.close();
    let data = context
        .get_image_data(0.0, 0.0, f64::from(width), f64::from(height))
        .ok()?;
    TransferImage::new(width, height, data.data().0).ok()
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
        let fallback = Closure::once_into_js(|| {
            if let Some(chord) = PENDING_PASTE.take() {
                chord.replay();
            }
        });
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            fallback.unchecked_ref(),
            PASTE_FALLBACK_MS,
        );
    }
    true
}

fn take_paste(event: web_sys::ClipboardEvent) {
    // Taking the chord also disarms the fallback replay.
    let Some(chord) = PENDING_PASTE.take() else {
        return;
    };
    event.prevent_default();
    let Some(data) = event.clipboard_data() else {
        chord.replay();
        return;
    };
    let read = |media_type: &str| {
        data.get_data(media_type)
            .ok()
            .filter(|text| !text.is_empty())
    };
    let mut contents = Contents {
        text: read("text/plain"),
        html: read("text/html"),
        image: None,
    };
    let items = data.items();
    let image = (0..items.length())
        .filter_map(|index| items.get(index))
        .find(|item| item.kind() == "file" && item.type_().starts_with("image/"))
        .and_then(|item| item.get_as_file().ok().flatten());
    let Some(image) = image else {
        contents.store();
        chord.replay();
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        contents.image = decode_image(image.into()).await;
        contents.store();
        chord.replay();
    });
}

impl Contents {
    /// Replaces the buffer with pasted contents; an empty paste keeps it.
    fn store(&self) {
        if let Some(transfer) = self.transfer() {
            BUFFER.set(Some(transfer));
        }
    }
}

impl Chord {
    fn replay(self) {
        for kind in ["keydown", "keyup"] {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(&self.key);
            init.set_code(&self.code);
            init.set_ctrl_key(self.ctrl);
            init.set_meta_key(self.meta);
            init.set_shift_key(self.shift);
            init.set_bubbles(true);
            init.set_cancelable(true);
            if let Ok(event) =
                web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(kind, &init)
            {
                let _ = self.target.dispatch_event(&event);
            }
        }
    }
}
