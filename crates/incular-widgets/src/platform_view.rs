//! Flutter's `PlatformViewSurface` and `PlatformViewController`
//! (`widgets/platform_view.dart`, `services/platform_views.dart`): a widget
//! that reserves layout and hit-test area for a native view and forwards
//! the pointers that land in it.
//!
//! Incular draws nothing for the view. After each frame the runtime reports
//! where every mounted view is ([`PlatformViewCommand::Place`]) together
//! with the commands its controller queued, and the host shows the native
//! view there. See [`incular_platform::PlatformViewCompositing`] for what a
//! WGPU host can and cannot composite.

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    fmt,
    rc::Rc,
};

use incular_config::TextDirection;
use incular_core::Rect;
use incular_gestures::RawPointerEvent;
use incular_platform::{
    PlatformViewCommand, PlatformViewId, PlatformViewPointerEvent, next_platform_view_id,
};

use crate::{HitTestBehavior, Listener, MetaData, SizedBox, Widget};

/// Flutter's `PlatformViewHitTestBehavior`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlatformViewHitTestBehavior {
    /// The view receives pointers in its bounds and hides what is behind it.
    #[default]
    Opaque,
    /// The view receives pointers in its bounds and so does what is behind
    /// it.
    Translucent,
    /// The view receives no pointers; what is behind it does.
    Transparent,
}

struct ControllerState {
    id: PlatformViewId,
    view_type: String,
    layout_direction: Cell<TextDirection>,
    disposed: Cell<bool>,
    commands: RefCell<VecDeque<PlatformViewCommand>>,
}

/// Flutter's `PlatformViewController`: one native view, identified by an
/// id, and the commands for it that the host has not received yet.
///
/// Clones share the view. Commands queue until the runtime of the window
/// that shows the view collects them, so a view must be mounted in a
/// [`PlatformViewSurface`] to be created on a host.
#[derive(Clone)]
pub struct PlatformViewController {
    state: Rc<ControllerState>,
}

impl PlatformViewController {
    /// Flutter `PlatformViewsService.initSurfaceAndroidView` and its
    /// siblings: a new view of the host's registered `view_type`, created
    /// with `creation_params` when it is first mounted.
    #[must_use]
    pub fn new(
        view_type: impl Into<String>,
        creation_params: Vec<u8>,
        layout_direction: TextDirection,
    ) -> Self {
        let id = next_platform_view_id();
        let view_type = view_type.into();
        let create = PlatformViewCommand::Create {
            id,
            view_type: view_type.clone(),
            creation_params,
            layout_direction,
        };
        Self {
            state: Rc::new(ControllerState {
                id,
                view_type,
                layout_direction: Cell::new(layout_direction),
                disposed: Cell::new(false),
                commands: RefCell::new(VecDeque::from([create])),
            }),
        }
    }

    /// Flutter `viewId`.
    #[must_use]
    pub fn id(&self) -> PlatformViewId {
        self.state.id
    }

    /// The view factory name.
    #[must_use]
    pub fn view_type(&self) -> &str {
        &self.state.view_type
    }

    /// The layout direction last set.
    #[must_use]
    pub fn layout_direction(&self) -> TextDirection {
        self.state.layout_direction.get()
    }

    /// Whether [`Self::dispose`] was called.
    #[must_use]
    pub fn is_disposed(&self) -> bool {
        self.state.disposed.get()
    }

    /// Flutter `dispatchPointerEvent`: forwards a pointer event in the
    /// view's coordinates.
    pub fn dispatch_pointer_event(&self, event: PlatformViewPointerEvent) {
        self.queue(PlatformViewCommand::Pointer {
            id: self.state.id,
            event,
        });
    }

    /// Flutter `setLayoutDirection`.
    pub fn set_layout_direction(&self, direction: TextDirection) {
        if self.state.layout_direction.replace(direction) != direction {
            self.queue(PlatformViewCommand::SetDirection {
                id: self.state.id,
                direction,
            });
        }
    }

    /// Flutter `clearFocus`.
    pub fn clear_focus(&self) {
        self.queue(PlatformViewCommand::ClearFocus { id: self.state.id });
    }

    /// Flutter `dispose`: the native view goes away. Later commands are
    /// dropped. A view disposed before the host received its `Create` never
    /// reaches the host.
    pub fn dispose(&self) {
        if self.state.disposed.replace(true) {
            return;
        }
        let mut commands = self.state.commands.borrow_mut();
        if matches!(commands.front(), Some(PlatformViewCommand::Create { .. })) {
            commands.clear();
        } else {
            commands.push_back(PlatformViewCommand::Dispose { id: self.state.id });
        }
    }

    /// Takes the commands queued since the last call, oldest first. The
    /// runtime collects them; hosts receive them from the runtime.
    #[must_use]
    pub fn take_commands(&self) -> Vec<PlatformViewCommand> {
        self.state.commands.borrow_mut().drain(..).collect()
    }

    fn queue(&self, command: PlatformViewCommand) {
        if !self.state.disposed.get() {
            self.state.commands.borrow_mut().push_back(command);
        }
    }
}

impl PartialEq for PlatformViewController {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl fmt::Debug for PlatformViewController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformViewController")
            .field("id", &self.state.id)
            .field("view_type", &self.state.view_type)
            .field("disposed", &self.state.disposed.get())
            .finish_non_exhaustive()
    }
}

/// The [`MetaData`] payload that marks a platform view's area.
pub(crate) struct PlatformViewMarker(pub(crate) PlatformViewController);

/// Flutter's `PlatformViewSurface`: takes the biggest size its constraints
/// allow (`sizedByParent`, `constraints.biggest`), paints nothing, and
/// forwards every pointer event in its area to the controller according to
/// `hit_test_behavior`.
///
/// Flutter forwards only the pointers of gestures its `gestureRecognizers`
/// win in the arena; this surface forwards all raw pointers in its area,
/// without arena arbitration, like a `Listener`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformViewSurface {
    controller: PlatformViewController,
    hit_test_behavior: PlatformViewHitTestBehavior,
}

impl PlatformViewSurface {
    /// A surface for `controller`'s view.
    #[must_use]
    pub fn new(controller: PlatformViewController) -> Self {
        Self {
            controller,
            hit_test_behavior: PlatformViewHitTestBehavior::Opaque,
        }
    }

    /// Flutter `hitTestBehavior`.
    #[must_use]
    pub fn hit_test_behavior(mut self, behavior: PlatformViewHitTestBehavior) -> Self {
        self.hit_test_behavior = behavior;
        self
    }
}

fn forward(controller: &PlatformViewController) -> impl Fn(RawPointerEvent) + 'static {
    let controller = controller.clone();
    move |event: RawPointerEvent| {
        controller.dispatch_pointer_event(PlatformViewPointerEvent {
            pointer: event.pointer,
            kind: event.kind,
            phase: event.phase,
            local_position: event.local_position,
            buttons: event.buttons,
        });
    }
}

impl From<PlatformViewSurface> for Widget {
    fn from(surface: PlatformViewSurface) -> Self {
        let controller = surface.controller;
        let area = SizedBox::new().width(f32::INFINITY).height(f32::INFINITY);
        let behavior = match surface.hit_test_behavior {
            PlatformViewHitTestBehavior::Opaque => HitTestBehavior::Opaque,
            PlatformViewHitTestBehavior::Translucent => HitTestBehavior::Translucent,
            // No listener: the area takes no pointers, so what is behind
            // it receives them.
            PlatformViewHitTestBehavior::Transparent => {
                return MetaData::new(PlatformViewMarker(controller))
                    .child(area)
                    .into();
            }
        };
        let listener = Listener::new(area)
            .behavior(behavior)
            .on_pointer_down(forward(&controller))
            .on_pointer_move(forward(&controller))
            .on_pointer_hover(forward(&controller))
            .on_pointer_up(forward(&controller))
            .on_pointer_cancel(forward(&controller));
        // Raw hit testing lets siblings behind a listener be hit; the
        // marker's behavior decides whether they are.
        MetaData::new(PlatformViewMarker(controller))
            .behavior(behavior)
            .child(listener)
            .into()
    }
}

/// Where a mounted platform view is after layout, in window coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformViewPlacement {
    /// The view.
    pub controller: PlatformViewController,
    /// The view's bounds.
    pub bounds: Rect,
    /// The part of `bounds` inside every ancestor's bounds, which is also
    /// where hit testing reaches it; `None` when nothing is left.
    pub visible_bounds: Option<Rect>,
}
