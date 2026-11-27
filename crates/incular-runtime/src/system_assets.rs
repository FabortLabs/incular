//! Flutter-compatible application system notifications.

use crate::Application;
use incular_assets::root_asset_bundle;
use incular_image::{clear_shared_bundle_image_cache, clear_shared_memory_image_cache};

#[derive(serde::Deserialize)]
struct SystemNotice {
    #[serde(rename = "type")]
    kind: String,
}

impl Application {
    /// Drops cached root asset strings and structured values on a host
    /// memory warning, and frees retained decoded bundle and memory images. Handles
    /// still used by mounted widgets remain valid.
    pub fn handle_memory_pressure(&self) {
        root_asset_bundle().clear();
        clear_shared_bundle_image_cache();
        clear_shared_memory_image_cache();
    }

    /// Handles the Flutter-compatible JSON system notifications for memory
    /// pressure and platform font changes. Unknown channels, kinds and
    /// malformed data can be handled by the caller.
    pub fn handle_system_message(&self, channel: &str, payload: &[u8]) -> bool {
        if channel != "flutter/system" {
            return false;
        }
        let Ok(notice) = serde_json::from_slice::<SystemNotice>(payload) else {
            return false;
        };
        match notice.kind.as_str() {
            "memoryPressure" => self.handle_memory_pressure(),
            "fontsChange" => incular_text::TextEngine::notify_system_fonts_changed(),
            _ => return false,
        }
        true
    }
}
