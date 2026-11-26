//! Application service delivery to the shared root asset bundle.

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

    /// Handles the Flutter-compatible JSON system notification for memory
    /// pressure. Other channels, unknown kinds and malformed data do not
    /// affect caches and can be handled by the caller.
    pub fn handle_system_message(&self, channel: &str, payload: &[u8]) -> bool {
        if channel != "flutter/system"
            || !serde_json::from_slice::<SystemNotice>(payload)
                .is_ok_and(|message| message.kind == "memoryPressure")
        {
            return false;
        }
        self.handle_memory_pressure();
        true
    }
}
