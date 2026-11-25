//! Flutter's `SystemChrome.setSystemUIOverlayStyle` and the
//! `RenderView._updateSystemChrome` probe that applies annotated styles.
//!
//! The runtime keeps one pending style per window. Each frame an annotation
//! at the status bar (and, as Flutter does on Android, at the navigation bar)
//! overrides it; the host drains the result once per frame and the runtime
//! reports only styles that differ from the last one it handed out.

use super::*;
use incular_platform::SystemUiOverlayStyle;

impl Runtime {
    /// Flutter's `SystemChrome.setSystemUIOverlayStyle`: asks the host to draw
    /// the status and navigation bars in `style`. A frame whose annotations
    /// find a style replaces this request.
    pub fn set_system_ui_overlay_style(&mut self, style: SystemUiOverlayStyle) {
        if self.system_ui_overlay_style.is_none()
            && self.system_ui_overlay_style_applied == Some(style)
        {
            return;
        }
        self.system_ui_overlay_style = Some(style);
        // An imperative change must reach the host even when no widgets
        // are rebuilding or animating. The frame also coalesces requests.
        self.request_frame();
    }

    /// Drains the overlay style the host should apply now. Returns `None`
    /// when nothing was requested since the last call, or when the request
    /// equals the style already handed out.
    pub fn take_system_ui_overlay_style(&mut self) -> Option<SystemUiOverlayStyle> {
        let style = self.system_ui_overlay_style.take()?;
        if self.system_ui_overlay_style_applied == Some(style) {
            return None;
        }
        self.system_ui_overlay_style_applied = Some(style);
        Some(style)
    }

    /// Probes the composited annotations at the two system-bar points, in
    /// logical pixels: the centre of the status bar (`padding.top / 2`) and
    /// the centre of the navigation bar (`bottom - 1 - padding.bottom / 2`,
    /// the last drawn row). Flutter reads the navigation bar only on Android;
    /// hosts without one never drain the navigation-bar fields.
    pub(crate) fn update_system_ui_overlay_style_from_annotations(&mut self) {
        let (center_x, status_y, nav_y) = {
            let environment = self.environment.borrow();
            let viewport = environment.viewport;
            let padding = environment.view_padding;
            let center_x = viewport.width / 2.;
            (
                center_x,
                padding.top / 2.,
                viewport.height - 1. - padding.bottom / 2.,
            )
        };
        let compositor = self.tree.compositor_layers();
        let upper =
            compositor.find_annotation::<SystemUiOverlayStyle>(Offset::new(center_x, status_y));
        let lower =
            compositor.find_annotation::<SystemUiOverlayStyle>(Offset::new(center_x, nav_y));
        if let Some(style) = SystemUiOverlayStyle::merge_annotations(upper, lower) {
            self.system_ui_overlay_style = Some(style);
        }
    }
}
