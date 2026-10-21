//! Browser variant of the desktop clipboard.
//!
//! The page clipboard (`navigator.clipboard`) is asynchronous and gated on a
//! user gesture, while [`Clipboard`] reads and writes synchronously. Rather
//! than fabricate success, every format is reported unsupported and each
//! operation fails with a typed error.

use incular_platform::{
    CapabilitySupport, Clipboard, ClipboardCapabilities, ClipboardError, ClipboardWriteReport,
    DataTransfer, TransferFormatCapabilities, TransferReadRequest,
};

const UNAVAILABLE: &str = "synchronous clipboard access is unavailable in browsers";

pub(crate) struct DesktopClipboard;

impl DesktopClipboard {
    pub(crate) const fn new() -> Self {
        Self
    }

    pub(crate) const fn native_capabilities(&self) -> ClipboardCapabilities {
        ClipboardCapabilities {
            read: TransferFormatCapabilities::all(CapabilitySupport::Unsupported),
            write: TransferFormatCapabilities::all(CapabilitySupport::Unsupported),
            lazy_write: CapabilitySupport::Unsupported,
        }
    }
}

impl Clipboard for DesktopClipboard {
    fn capabilities(&self) -> ClipboardCapabilities {
        self.native_capabilities()
    }

    fn read(&mut self, _request: TransferReadRequest) -> Result<DataTransfer, ClipboardError> {
        Err(ClipboardError::Backend(UNAVAILABLE.to_owned()))
    }

    fn write(&mut self, _transfer: DataTransfer) -> Result<ClipboardWriteReport, ClipboardError> {
        Err(ClipboardError::Backend(UNAVAILABLE.to_owned()))
    }
}
