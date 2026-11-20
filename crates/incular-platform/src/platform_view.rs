//! Flutter's platform views (`services/platform_views.dart` and
//! `rendering/platform_view.dart`): native views, such as a map, a web view
//! or a camera preview, that a host shows inside an Incular layout.
//!
//! The portable half lives here: view identities, the commands the runtime
//! sends to a host, and what a host can do with them. Incular reserves the
//! view's layout and hit-test area and forwards pointers into it; the host
//! creates, positions and composites the native view.
//!
//! What cannot be composited on WGPU: Flutter's engine either renders a
//! native view into a texture that it draws in the layer tree (Android
//! virtual displays and texture layers) or splits its own output into
//! overlay surfaces above and below each native view (hybrid composition on
//! Android and iOS). Incular's WGPU renderer has neither: it owns one
//! surface per window and cannot sample another process's or toolkit's
//! pixels. A host that supports platform views can therefore only place a
//! native view above the Incular surface ([`PlatformViewCompositing::AboveSurface`]):
//! Incular content painted over the view's area stays hidden behind it,
//! clips and transforms other than translation are reduced to the view's
//! visible bounds, and opacity and filters do not apply. The desktop and web
//! hosts implement none of this and report
//! [`PlatformViewCompositing::Unsupported`].

use incular_config::TextDirection;
use incular_core::{IdGenerator, Offset, PointerDeviceKind, PointerPhase, Rect};

/// Flutter's platform view `id`: identifies one native view between the
/// framework and the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlatformViewId(pub u64);

static NEXT_PLATFORM_VIEW_ID: IdGenerator = IdGenerator::starting_at(0);

/// Flutter `PlatformViewsRegistry.getNextPlatformViewId`: a process-unique
/// identity for a new platform view, counting from 0.
#[must_use]
pub fn next_platform_view_id() -> PlatformViewId {
    PlatformViewId(NEXT_PLATFORM_VIEW_ID.next())
}

/// How a host can composite platform views with the Incular surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlatformViewCompositing {
    /// The host shows no native views. It drops platform view commands, and
    /// the view's area stays empty. This is every current Incular host.
    #[default]
    Unsupported,
    /// The host places each native view above the Incular surface at its
    /// reported bounds. Incular content cannot cover it, and only the
    /// visible bounds of clips apply. See the module documentation.
    AboveSurface,
}

/// One pointer event forwarded to a native view, in the view's logical
/// coordinates (Flutter's `AndroidMotionEvent` and the `touch` message).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformViewPointerEvent {
    /// Stable for the duration of one pointer sequence.
    pub pointer: u64,
    /// The device category.
    pub kind: PointerDeviceKind,
    /// The phase.
    pub phase: PointerPhase,
    /// Position relative to the view's top-left corner.
    pub local_position: Offset,
    /// Flutter-compatible button mask.
    pub buttons: u32,
}

/// Commands the runtime sends to a host's platform view support, named
/// after Flutter's `flutter/platform_views` methods.
#[derive(Clone, Debug, PartialEq)]
pub enum PlatformViewCommand {
    /// Flutter `create`: make a native view of the registered `view_type`.
    Create {
        id: PlatformViewId,
        /// The view factory name the host registered.
        view_type: String,
        /// Encoded creation parameters, opaque to Incular.
        creation_params: Vec<u8>,
        /// Layout direction for the native view.
        layout_direction: TextDirection,
    },
    /// Flutter `resize` and `offset`, plus visibility: where the view is in
    /// the window, in logical pixels, after layout. `visible` is `false`
    /// while the view's widget is out of the tree or clipped away entirely.
    Place {
        id: PlatformViewId,
        /// The view's bounds in window coordinates.
        bounds: Rect,
        /// The part of `bounds` that ancestor clips leave visible.
        visible_bounds: Rect,
        /// Whether to show the view.
        visible: bool,
    },
    /// Flutter `touch`: a pointer event inside the view.
    Pointer {
        id: PlatformViewId,
        event: PlatformViewPointerEvent,
    },
    /// Flutter `setDirection`.
    SetDirection {
        id: PlatformViewId,
        direction: TextDirection,
    },
    /// Flutter `clearFocus`: the native view gives up input focus.
    ClearFocus { id: PlatformViewId },
    /// Flutter `dispose`: the native view goes away.
    Dispose { id: PlatformViewId },
}

impl PlatformViewCommand {
    /// The view the command is for.
    #[must_use]
    pub fn id(&self) -> PlatformViewId {
        match self {
            Self::Create { id, .. }
            | Self::Place { id, .. }
            | Self::Pointer { id, .. }
            | Self::SetDirection { id, .. }
            | Self::ClearFocus { id }
            | Self::Dispose { id } => *id,
        }
    }
}
