//! Native per-window ownership. Raw-handle users precede the retained window.
use crate::{
    environment::DesktopEnvironmentProvider, external_drag::ExternalFileDragState,
    input::WindowInputState, pointer::NativeCursorCoordinator,
};
use accesskit_winit::Adapter as AccessKitAdapter;
use incular_accessibility::{AccessKitProjection, MobileAccessibilityProjection};
use incular_host::HostSurface;
use incular_platform::NoopContentSensitivityBackend;
use std::sync::Arc;
use winit::window::Window;
/// One AccessKit adapter/projection pair belongs to exactly one native window.
/// The projection is pure retained-tree state; `Adapter` owns the OS bridge.
pub(crate) struct NativeAccessibilityState {
    pub(crate) adapter: AccessKitAdapter,
    pub(crate) projection: AccessKitProjection,
    pub(crate) active: bool,
    /// Semantics presented by the platform services instead of AccessKit
    /// (a browser page), active once they ask for them.
    pub(crate) service_projection: MobileAccessibilityProjection,
    pub(crate) service_active: bool,
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
