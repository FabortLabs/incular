//! Native per-window ownership. Raw-handle users precede the retained window.
use crate::{
    environment::DesktopEnvironmentProvider, external_drag::ExternalFileDragState,
    input::WindowInputState, pointer::NativeCursorCoordinator,
};
use accesskit_winit::Adapter as AccessKitAdapter;
use incular_accessibility::{AccessKitProjection, MobileAccessibilityProjection};
use incular_host::HostSurface;
use incular_platform::{NoopContentSensitivityBackend, WindowId};
use incular_runtime::{Application, SemanticsHandle};
use std::sync::Arc;
use winit::window::Window;
/// One AccessKit adapter/projection pair belongs to exactly one native window.
/// The projection is pure retained-tree state; `Adapter` owns the OS bridge.
pub(crate) struct NativeAccessibilityState {
    pub(crate) adapter: AccessKitAdapter,
    pub(crate) projection: AccessKitProjection,
    pub(crate) active: bool,
    pub(crate) service: ServiceAccessibility,
}

/// The platform service (e.g. a browser page) and AccessKit are independent
/// consumers of the same retained semantics. One must not deactivate the
/// tree on behalf of the other.
#[derive(Default)]
pub(crate) struct ServiceAccessibility {
    projection: MobileAccessibilityProjection,
    demand: Option<SemanticsHandle>,
}

impl ServiceAccessibility {
    /// Returns true only when a new consumer needs its initial snapshot.
    pub(crate) fn activate(&mut self, application: &mut Application, id: WindowId) -> bool {
        if self.demand.is_some() {
            return false;
        }
        let Some(demand) = application.ensure_semantics(id) else {
            return false;
        };
        self.projection.activate();
        self.demand = Some(demand);
        true
    }

    pub(crate) fn projection(&mut self) -> Option<&mut MobileAccessibilityProjection> {
        self.demand.as_ref().map(|_| &mut self.projection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use incular_config::Constraints;
    use incular_core::{Size, time::Instant};
    use incular_widgets::{Semantics, SizedBox};

    #[test]
    fn platform_service_keeps_publishing_after_accesskit_deactivates() {
        let mut application =
            Application::new(|_| Semantics::new(SizedBox::expand()).label("shared").into())
                .unwrap();
        let window = application.primary_window();
        let bounds = Constraints::tight(Size::new(400., 300.));
        // An AccessKit consumer has already collected the tree and is idle.
        application.set_semantics_enabled(window, true);
        application
            .run_window_frame_at(window, bounds, Instant::now())
            .unwrap();
        assert!(!application.frame_requested(window));

        let mut service = ServiceAccessibility::default();
        assert!(service.projection().is_none());
        assert!(service.activate(&mut application, window));
        assert!(!service.activate(&mut application, window));
        // The initial service redraw can consume the already retained tree.
        let first = application
            .sync_semantics_projection(window, service.projection().unwrap())
            .unwrap();
        let [node] = first.nodes.as_slice() else {
            panic!("expected the live window label");
        };
        assert_eq!(node.semantic.label.as_deref(), Some("shared"));
        application.set_semantics_enabled(window, false);
        assert_eq!(application.semantics_enabled(window), Some(true));
        assert!(!application.frame_requested(window));
        assert!(
            application
                .sync_semantics_projection(window, service.projection().unwrap())
                .is_none(),
            "AccessKit deactivation must neither remove nor republish service nodes"
        );
        drop(service);
        assert_eq!(application.semantics_enabled(window), Some(false));
        assert!(application.frame_requested(window));
    }
}
pub(crate) struct NativeWindowState {
    pub(crate) surface: HostSurface,
    pub(crate) input: WindowInputState,
    pub(crate) native_cursor: NativeCursorCoordinator,
    pub(crate) environment: DesktopEnvironmentProvider,
    pub(crate) external_file_drag: ExternalFileDragState,
    pub(crate) accessibility: NativeAccessibilityState,
    pub(crate) content_sensitivity: NoopContentSensitivityBackend,
    /// Must outlive every field that was created from this window's raw
    /// handles. Struct fields drop in declaration order, so keep it last.
    pub(crate) window: Arc<Window>,
}
