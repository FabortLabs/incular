//! Applies Flutter's mobile-only return-key hints to a browser input.

use crate::TextInputAction;

/// Applies or clears the hint for the current editor client. A reused
/// textarea must not keep the earlier client's hint on unsupported actions.
pub fn update_dom_enter_key_hint(element: &web_sys::Element, action: TextInputAction) {
    let hint = action.web_enter_key_hint().filter(|_| is_mobile_keyboard());
    if let Some(hint) = hint {
        let _ = element.set_attribute("enterkeyhint", hint);
    } else {
        let _ = element.remove_attribute("enterkeyhint");
    }
}

fn is_mobile_keyboard() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    let platform = navigator.platform().unwrap_or_default();
    if platform.starts_with("Mac") {
        // iPadOS can report a desktop Mac platform.
        return navigator.max_touch_points() > 2;
    }
    let casefolded = platform.to_ascii_lowercase();
    ["iphone", "ipad", "ipod"]
        .iter()
        .any(|name| casefolded.contains(name))
        || navigator
            .user_agent()
            .is_ok_and(|agent| agent.contains("Android"))
}
