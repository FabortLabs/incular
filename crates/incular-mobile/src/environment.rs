//! The system environment a mobile shell reports.

use incular_config::{Brightness, EdgeInsets, Locale, RuntimeEnvironment};
use incular_platform::WindowMetrics;

/// What the native shell knows about the system, in logical pixels.
///
/// Inset ownership follows `SystemEnvironmentPreferences`: `view_padding` is
/// the persistent obstruction (system bars, cutouts, home indicator),
/// `view_insets` the transient one (the soft keyboard), and `safe_insets` what
/// is usable right now: per edge `max(0, view_padding - view_insets)`, as
/// Flutter's `MediaQuery.padding`. A shown keyboard covers the padding it
/// overlaps, so it reduces the safe edge rather than adding to it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MobileEnvironment {
    pub view_padding: EdgeInsets,
    pub view_insets: EdgeInsets,
    pub system_gesture_insets: EdgeInsets,
    pub brightness: Option<Brightness>,
    pub text_scale: Option<f32>,
    pub locales: Option<Vec<Locale>>,
    pub reduced_motion: Option<bool>,
    pub bold_text: Option<bool>,
}

impl MobileEnvironment {
    /// The complete runtime environment for a view with `metrics`.
    #[must_use]
    pub fn snapshot(&self, metrics: WindowMetrics) -> RuntimeEnvironment {
        let defaults = RuntimeEnvironment::default();
        let padding = self.view_padding;
        let insets = self.view_insets;
        let safe_insets = EdgeInsets {
            left: (padding.left - insets.left).max(0.0),
            top: (padding.top - insets.top).max(0.0),
            right: (padding.right - insets.right).max(0.0),
            bottom: (padding.bottom - insets.bottom).max(0.0),
        };
        let mut environment = RuntimeEnvironment {
            viewport: metrics.logical_size(),
            physical_width: metrics.physical_size.width,
            physical_height: metrics.physical_size.height,
            scale_factor: metrics.scale_factor,
            brightness: self.brightness.unwrap_or(defaults.brightness),
            text_scale: self.text_scale.unwrap_or(defaults.text_scale),
            reduced_motion: self.reduced_motion.unwrap_or(defaults.reduced_motion),
            bold_text: self.bold_text.unwrap_or(defaults.bold_text),
            view_padding: self.view_padding,
            view_insets: self.view_insets,
            safe_insets,
            system_gesture_insets: self.system_gesture_insets,
            window_focused: true,
            ..defaults
        };
        if let Some(locales) = &self.locales {
            environment.locales.clone_from(locales);
        }
        environment.input.touch = true;
        environment.normalized()
    }
}
