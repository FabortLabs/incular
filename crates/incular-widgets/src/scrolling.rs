use incular_core::time::Instant;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, Curve, Tween};
use incular_config::{Axis, Clip, Constraints, EdgeInsets, WidgetDefaults};
use incular_core::{DragStartBehavior, Size};
use incular_scroll::{
    MeasuredExtentIndex, ScrollController, ScrollNotification, ScrollNotificationSubscription,
    ScrollNotificationType, ScrollPhysics, SliverConstraints, SliverGeometry,
};
use typed_builder::TypedBuilder;

use crate::drag_drop::DragDropContext;
use crate::tree::WidgetKind;
use crate::{Column, DragTarget, Draggable, Padding, Row, SizedBox, Widget};

type SliverLayoutBuilderFn = Rc<dyn Fn(SliverConstraints) -> Widget>;

const DEFAULT_LAZY_ITEM_EXTENT: f32 = WidgetDefaults::DEFAULT.lazy_item_extent;
const DEFAULT_SLIVER_CACHE_EXTENT: f32 = WidgetDefaults::DEFAULT.sliver_cache_extent;

mod geometry;
mod layout;
mod notifications;
mod reorderable;
mod scroll_views;
mod sliver_descriptors;
mod sliver_lists;

pub use geometry::{
    RenderSliver, SliverChildId, SliverChildLayout, SliverChildPlacement, SliverLayout,
    SliverScrollDependency, SliverViewportConfig,
};
pub(crate) use geometry::{SliverViewportDelegate, SliverViewportLayout};

pub use layout::SliverOverlapHandle;
use layout::{
    BoxRenderSliver, FillRemainingRenderSliver, FixedExtentRenderSliver,
    FloatingHeaderRenderSliver, GridRenderSliver, HeaderPresentation, HeaderRenderSliver,
    HeaderScrollState, HeaderSnapFrame, LayoutBuilderRenderSliver, NaturalHeaderExtent,
    NaturalHeaderRenderSliver, OverlapAbsorberRenderSliver, OverlapInjectorRenderSliver,
    PaddingRenderSliver, ResizingHeaderRenderSliver, SequenceRenderSliver,
    SequenceViewportDelegate, SliverViewportOptions, VariableExtentRenderSliver,
    ViewportExtentRenderSliver, WidgetWrapRenderSliver, single_sliver_viewport,
    single_sliver_viewport_with_options, sliver_child_constraints, snap_activity_subscription,
    widget_main_extent_hint,
};
pub(crate) use layout::{adopt_reconciled_measurement, invalidate_transferred_measurement};

pub use notifications::*;
pub use reorderable::{
    ReorderableDelayedDragStartListener, ReorderableDragStartListener, ReorderableList,
};
pub use scroll_views::*;
pub use sliver_descriptors::*;
pub use sliver_lists::*;

/// Flutter `ScrollPosition.restoreScrollOffset`/`saveScrollOffset` for a
/// scroll view keyed by `key`: with `keepScrollOffset`, the offset is read
/// from the nearest [`PageStorage`](crate::PageStorage) when the view is
/// built and written back whenever scrolling settles.
fn keep_scroll_offset(
    view: Widget,
    controller: &ScrollController,
    key: Option<crate::PageStorageKey>,
) -> Widget {
    let Some(key) = key.filter(|_| controller.keep_scroll_offset()) else {
        return view;
    };
    let controller = controller.clone();
    let subscription: Rc<
        RefCell<Option<(crate::PageStorageBucket, ScrollNotificationSubscription)>>,
    > = Rc::new(RefCell::new(None));
    Widget::stateful_builder(Rc::new(Cell::new(0)), move |context| {
        let bucket = crate::PageStorage::maybe_of(context);
        let current = subscription
            .borrow()
            .as_ref()
            .map(|(bucket, _)| bucket.clone());
        if bucket != current {
            *subscription.borrow_mut() = bucket.map(|bucket| {
                if let Some(offset) = bucket.read_state(&key).and_then(|value| value.as_f64()) {
                    controller.deferred_jump_to(offset as f32);
                }
                let listener = {
                    let bucket = bucket.clone();
                    let key = key.clone();
                    let controller = controller.clone();
                    controller.clone().add_listener(move |notification| {
                        // `didEndScroll`: an activity ended, or an idle jump
                        // moved the position.
                        let settled = match notification.kind {
                            incular_scroll::ScrollNotificationType::End => true,
                            incular_scroll::ScrollNotificationType::Update => {
                                controller.current_activity_id().is_none()
                            }
                            _ => false,
                        };
                        if settled {
                            bucket.write_state(&key, f64::from(controller.offset()));
                        }
                        false
                    })
                };
                (bucket, listener)
            });
        }
        view.clone()
    })
}
