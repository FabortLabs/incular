use incular_platform::{TextInputAction as Action, update_dom_enter_key_hint};
use wasm_bindgen::prelude::*;
use web_sys::Element;

#[wasm_bindgen]
pub fn apply_action(element: &Element, action: u8) {
    let action = [
        Action::Done,
        Action::ContinueAction,
        Action::Join,
        Action::Unspecified,
        Action::Newline,
        Action::Previous,
        Action::EmergencyCall,
        Action::Route,
        Action::None,
    ][usize::from(action)];
    update_dom_enter_key_hint(element, action);
}
