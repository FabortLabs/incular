//! The status and navigation bar style, following Flutter's Android embedding
//! (`PlatformPlugin.setSystemChromeSystemUIOverlayStyle`). Kotlin applies it
//! on the main thread in `IncularNative.setSystemUiOverlayStyle`.
//!
//! Icon brightness crosses as `-1` (unset), `0` (light icons) or `1` (dark
//! icons). Colours cross as ARGB; `flags` says which colours are set.

use super::java::{self, Arg};
use incular_config::Brightness;
use incular_core::Color;
use incular_platform::SystemUiOverlayStyle;
use jni_sys::jint;

const ICON_UNSET: jint = -1;
const ICON_LIGHT: jint = 0;
const ICON_DARK: jint = 1;
/// `flags` bit: `statusColor` is set.
const FLAG_STATUS_COLOR: jint = 1;
/// `flags` bit: `navColor` is set.
const FLAG_NAVIGATION_COLOR: jint = 2;

/// Applies the style's status and navigation bar fields on the Kotlin side.
pub(super) fn apply(style: SystemUiOverlayStyle) {
    let mut flags = 0;
    let status_color = style.status_bar_color.map_or(0, |color| {
        flags |= FLAG_STATUS_COLOR;
        argb(color)
    });
    let navigation_color = style.system_navigation_bar_color.map_or(0, |color| {
        flags |= FLAG_NAVIGATION_COLOR;
        argb(color)
    });
    java::call_void(
        c"setSystemUiOverlayStyle",
        c"(IIIII)V",
        &[
            Arg::Int(icons(style.status_bar_icon_brightness)),
            Arg::Int(icons(style.system_navigation_bar_icon_brightness)),
            Arg::Int(status_color),
            Arg::Int(navigation_color),
            Arg::Int(flags),
        ],
    );
}

/// Flutter's icon brightness: `Brightness::Dark` draws dark icons.
fn icons(brightness: Option<Brightness>) -> jint {
    match brightness {
        None => ICON_UNSET,
        Some(Brightness::Light) => ICON_LIGHT,
        Some(Brightness::Dark) => ICON_DARK,
    }
}

/// `android.graphics.Color` ARGB as a Java `int`.
fn argb(color: Color) -> jint {
    jint::from_be_bytes([color.alpha, color.red, color.green, color.blue])
}
