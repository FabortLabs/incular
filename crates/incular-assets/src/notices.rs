//! Loads the application license notices from an explicit root asset bundle.

use std::{
    io::Read,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex, PoisonError},
    task::{Context, Poll, Waker},
};

use flate2::read::MultiGzDecoder;
use futures_core::Stream;
use incular_core::licenses::{LicenseEntry, LicenseRegistry};

use crate::{AssetBundle, AssetError};

const NATIVE_KEY: &str = "NOTICES.Z";
const WEB_KEY: &str = "NOTICES";
const MAX_BYTES: usize = 32 * 1024 * 1024;
const SEPARATOR: &str = concat!(
    "\n",
    "--------------------------------------------------------------------------------",
    "\n",
);

/// Wire format used by the application asset pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LicenseNoticeFormat {
    /// Bundled gzip data in `NOTICES.Z`.
    NativeGzip,
    /// The web host already handles compression; read uncached UTF-8
    /// from `NOTICES`.
    WebText,
}

impl LicenseNoticeFormat {
    #[must_use]
    pub const fn current_platform() -> Self {
        if cfg!(target_arch = "wasm32") {
            Self::WebText
        } else {
            Self::NativeGzip
        }
    }

    #[must_use]
    pub const fn asset_key(self) -> &'static str {
        match self {
            Self::NativeGzip => NATIVE_KEY,
            Self::WebText => WEB_KEY,
        }
    }
}

/// Reads and parses the selected packaged notice. The web text bypasses
/// any cached strings; the native path limits decompression to 32 MiB.
///
/// Use [register_license_notices] to defer loading until a consumer
/// actually polls the foundation registry.
///
/// # Errors
///
/// A missing, malformed, invalid UTF-8, or oversized notice is returned
/// as an asset error with the corresponding bundle key.
pub fn load_license_notices(
    bundle: &dyn AssetBundle,
    format: LicenseNoticeFormat,
) -> Result<Vec<LicenseEntry>, AssetError> {
    match format {
        LicenseNoticeFormat::WebText => parse_text(&bundle.load_string(WEB_KEY, false)?, WEB_KEY),
        LicenseNoticeFormat::NativeGzip => decode_bytes(&bundle.load(NATIVE_KEY)?, format),
    }
}

/// Adds a lazy notice collector to the foundation registry. Pass the root
/// bundle at application setup. Native hosts take the read/decompression
/// off the caller's thread; web hosts use the bundle's asynchronous byte
/// load and do not populate its text cache.
///
/// Bundle errors are delivered when the collector is polled. Other
/// registered collectors may still supply notices.
pub fn register_license_notices(
    bundle: Arc<dyn AssetBundle>,
    format: LicenseNoticeFormat,
    on_error: impl Fn(AssetError) + 'static,
) {
    let on_error = Rc::new(on_error);
    LicenseRegistry::add_license(move || NoticeStream {
        bundle: Some(bundle.clone()),
        format,
        on_error: on_error.clone(),
        result: Arc::new(Mutex::new(NoticeResult::default())),
        entries: None,
    });
}

fn parse_text(text: &str, key: &str) -> Result<Vec<LicenseEntry>, AssetError> {
    if text.len() > MAX_BYTES {
        return Err(too_large(key));
    }
    Ok(text
        .split(SEPARATOR)
        .map(|section| {
            if let Some((packages, body)) = section.split_once("\n\n") {
                LicenseEntry::with_line_breaks(packages.split('\n'), body)
            } else {
                LicenseEntry::with_line_breaks(std::iter::empty::<&str>(), section)
            }
        })
        .collect())
}

fn too_large(key: &str) -> AssetError {
    AssetError::Parse {
        key: key.to_owned(),
        message: "license notices exceed the 32 MiB expanded limit".into(),
    }
}

fn decode_bytes(
    bytes: &[u8],
    format: LicenseNoticeFormat,
) -> Result<Vec<LicenseEntry>, AssetError> {
    let key = format.asset_key();
    let decoded;
    let bytes = match format {
        LicenseNoticeFormat::WebText => bytes,
        LicenseNoticeFormat::NativeGzip => {
            // A multi-member gzip is still one contiguous notices document.
            decoded = {
                let mut data = Vec::new();
                MultiGzDecoder::new(bytes)
                    .take(MAX_BYTES as u64 + 1)
                    .read_to_end(&mut data)
                    .map_err(|error| AssetError::Parse {
                        key: key.to_owned(),
                        message: error.to_string(),
                    })?;
                data
            };
            &decoded
        }
    };
    if bytes.len() > MAX_BYTES {
        return Err(too_large(key));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| AssetError::InvalidUtf8(key.to_owned()))?;
    parse_text(text, key)
}

#[derive(Default)]
struct NoticeResult {
    completed: Option<Result<Vec<LicenseEntry>, AssetError>>,
    waiter: Option<Waker>,
}

type SharedResult = Arc<Mutex<NoticeResult>>;

fn finish(result: &SharedResult, value: Result<Vec<LicenseEntry>, AssetError>) {
    let mut result = result.lock().unwrap_or_else(PoisonError::into_inner);
    result.completed = Some(value);
    let waiter = result.waiter.take();
    drop(result);
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

fn begin(bundle: Arc<dyn AssetBundle>, format: LicenseNoticeFormat, result: SharedResult) {
    let key = format.asset_key();
    let worker_result = result.clone();
    let load = move || {
        bundle.load_async(
            key,
            Box::new(move |bytes| {
                finish(
                    &worker_result,
                    bytes.and_then(|bytes| decode_bytes(&bytes, format)),
                );
            }),
        );
    };
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = std::thread::Builder::new()
        .name("incular-license-notices".into())
        .spawn(load)
    {
        finish(
            &result,
            Err(AssetError::Io {
                key: key.to_owned(),
                message: error.to_string(),
            }),
        );
    }
    #[cfg(target_arch = "wasm32")]
    load();
}

struct NoticeStream<F> {
    bundle: Option<Arc<dyn AssetBundle>>,
    format: LicenseNoticeFormat,
    on_error: Rc<F>,
    result: SharedResult,
    entries: Option<std::vec::IntoIter<LicenseEntry>>,
}

impl<F: Fn(AssetError)> Stream for NoticeStream<F> {
    type Item = LicenseEntry;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let stream = self.get_mut();
        if let Some(entries) = &mut stream.entries {
            return Poll::Ready(entries.next());
        }
        if let Some(bundle) = stream.bundle.take() {
            begin(bundle, stream.format, stream.result.clone());
        }
        let completed = {
            let mut result = stream.result.lock().unwrap_or_else(PoisonError::into_inner);
            match result.completed.take() {
                Some(completed) => completed,
                None => {
                    result.waiter = Some(context.waker().clone());
                    return Poll::Pending;
                }
            }
        };
        let entries = match completed {
            Ok(entries) => entries,
            Err(error) => {
                (stream.on_error)(error);
                Vec::new()
            }
        };
        let mut entries = entries.into_iter();
        let first = entries.next();
        stream.entries = Some(entries);
        Poll::Ready(first)
    }
}
