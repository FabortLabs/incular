use incular_core::time::Instant;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, Curve, Tween};
use incular_config::{Axis, Clip, Constraints, EdgeInsets, WidgetDefaults};
use incular_core::{DragStartBehavior, Rect, Size};
use incular_scroll::{
    MeasuredExtentIndex, ScrollController, ScrollNotification, ScrollNotificationSubscription,
    ScrollNotificationType, ScrollPhysics, SliverConstraints, SliverGeometry, SliverGrowthDirection,
};
use typed_builder::TypedBuilder;

use crate::drag_drop::DragDropContext;
use crate::tree::{Key as WidgetKey, WidgetKind};
use crate::{Column, Draggable, Padding, Row, SizedBox, Widget};

type FindChildIndex = Rc<dyn Fn(&WidgetKey) -> Option<usize>>;
struct ListIndexLookup {
    index: FindChildIndex,
    // An application callback observes all previously materialized keys.
    // A static list needs the map only to remap children it will park.
    include_active: bool,
}

type SliverLayoutBuilderFn = Rc<dyn Fn(SliverConstraints) -> Widget>;

/// How far a keyboard or programmatic scroll action should move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollIncrementType {
    #[default]
    Line,
    Page,
}

/// The kind of request and the scrollable's live geometry at invocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollIncrementDetails {
    pub increment_type: ScrollIncrementType,
    pub metrics: incular_scroll::ScrollMetrics,
}

/// Calculates a logical-pixel increment for a programmatic or keyboard scroll.
#[derive(Clone)]
pub struct ScrollIncrementCalculator(Rc<dyn Fn(ScrollIncrementDetails) -> f32>);

impl ScrollIncrementCalculator {
    #[must_use]
    pub fn new(calculate: impl Fn(ScrollIncrementDetails) -> f32 + 'static) -> Self {
        Self(Rc::new(calculate))
    }

    #[must_use]
    pub fn calculate(&self, details: ScrollIncrementDetails) -> f32 {
        (self.0)(details)
    }
}

impl PartialEq for ScrollIncrementCalculator {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

// Binds a low-level Scrollable's option through an application-supplied
// wrapper, without applying it to another controller nested below it.
#[derive(Clone)]
pub(crate) struct ScopedScrollIncrement {
    pub(crate) controller: ScrollController,
    pub(crate) calculator: ScrollIncrementCalculator,
}

/// A typed action request for the nearest retained scrollable. Invoke it
/// from a mounted element through the runtime action dispatcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScrollIntent {
    pub direction: incular_config::AxisDirection,
    pub increment_type: ScrollIncrementType,
}

impl ScrollIntent {
    #[must_use]
    pub const fn new(direction: incular_config::AxisDirection) -> Self {
        Self {
            direction,
            increment_type: ScrollIncrementType::Line,
        }
    }

    #[must_use]
    pub const fn increment_type(mut self, increment_type: ScrollIncrementType) -> Self {
        self.increment_type = increment_type;
        self
    }
}

const DEFAULT_LAZY_ITEM_EXTENT: f32 = WidgetDefaults::DEFAULT.lazy_item_extent;
const DEFAULT_SLIVER_CACHE_EXTENT: f32 = WidgetDefaults::DEFAULT.sliver_cache_extent;

mod child_delegates;
mod ensure_visible;
mod geometry;
mod layout;
mod notifications;
mod reorderable;
mod scroll_views;
mod sliver_descriptors;
mod sliver_decoration;
pub use sliver_decoration::{SliverBoxPaintConfiguration, SliverBoxPainter, SliverBoxPainterFactory};
mod sliver_fade;
mod sliver_paint_geometry;
pub(crate) mod sliver_layer;
pub(crate) mod sliver_clip;
pub use sliver_clip::{ClipOverlapBehavior, SliverClipRRect, SliverClipRect};
mod sliver_lists;
mod sliver_semantics;

#[doc(hidden)]
pub use geometry::SliverKeepAliveSource;
pub use geometry::{
    RenderSliver, ScrollCacheExtent, SliverChildId, SliverChildLayout, SliverChildPlacement,
    SliverLayout, SliverPaintOrder, SliverScrollDependency, SliverViewportConfig,
};
pub(crate) use geometry::{
    SliverAxisResolution, SliverKeyboardDismissal, SliverRevealGeometry, SliverViewportDelegate, SliverViewportLayout,
    sliver_paint_slots,
};

pub use layout::SliverOverlapHandle;
use layout::{
    BoxRenderSliver, EmptyRenderSliver, FillRemainingRenderSliver, FixedExtentRenderSliver,
    FloatingHeaderRenderSliver, GridRenderSliver, HeaderPresentation, HeaderRenderSliver,
    HeaderScrollState, HeaderSnapFrame, LayoutBuilderRenderSliver, NaturalHeaderExtent,
    NaturalHeaderRenderSliver, OverlapAbsorberRenderSliver, OverlapInjectorRenderSliver,
    PaddingRenderSliver, ResizingHeaderRenderSliver, SequenceRenderSliver,
    SequenceViewportDelegate, SliverViewportOptions, VariableExtentRenderSliver, ViewportCenter,
    ViewportExtentRenderSliver, WidgetWrapRenderSliver, single_sliver_viewport,
    single_sliver_viewport_with_options, sliver_child_constraints, snap_activity_subscription,
    widget_main_extent_hint,
};
pub(crate) use layout::{adopt_reconciled_measurement, invalidate_transferred_measurement};

use child_delegates::delegate_builder;
pub use child_delegates::{
    SemanticIndexCallback, SliverChildBuilderDelegate, SliverChildDelegate, SliverChildListDelegate,
};
pub use ensure_visible::{EnsureVisible, RevealedOffset, ScrollPositionAlignmentPolicy, ShowOnScreen};
pub use notifications::*;
pub use reorderable::{
    ReorderableDelayedDragStartListener, ReorderableDragStartListener, ReorderableList,
};
pub use scroll_views::*;
pub use sliver_descriptors::*;
pub use sliver_fade::{SliverAnimatedOpacity, SliverFadeTransition};
pub use sliver_lists::*;
pub use sliver_semantics::{SliverEnsureSemantics, SliverSemantics};
pub(crate) use sliver_semantics::{in_semantics_group, is_semantics_anchor};

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
