use incular_config::TargetPlatform;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn adaptive_platform() -> String {
    format!("{:?}", TargetPlatform::current())
}
