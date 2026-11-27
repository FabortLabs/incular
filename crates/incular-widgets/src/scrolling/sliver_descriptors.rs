use super::*;
use incular_core::finite_non_negative;

/// Unified sliver protocol. A sliver creates a retained render-sliver node.
pub trait Sliver {
    fn build(&self, controller: &ScrollController) -> Widget;

    /// Cross-axis groups use this flex when the sliver is wrapped in
    /// [`SliverCrossAxisExpanded`]. Ordinary slivers occupy one equal lane.
    fn cross_axis_flex(&self) -> Option<usize> {
        None
    }

    /// Builds a sliver with the owning viewport's axis and direction.  The
    /// default keeps existing custom slivers source-compatible; built-in
    /// headers override it so pinning also works in horizontal/reversed
    /// viewports.
    fn build_with_config(
        &self,
        controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Widget {
        self.build(controller)
    }

    /// Creates the retained sliver protocol implementation. The default is a
    /// true `SliverToBoxAdapter`, which preserves source compatibility for
    /// custom slivers while built-ins override it with lazy geometry-aware
    /// implementations.
    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(BoxRenderSliver::new(
            self.build_with_config(controller, axis, reverse),
        ))
    }
}

/// Adapts an ordinary box widget into a sliver.
#[derive(Clone, TypedBuilder)]
pub struct SliverToBoxAdapter {
    #[builder(setter(into))]
    child: Widget,
}

impl SliverToBoxAdapter {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl Sliver for SliverToBoxAdapter {
    fn build(&self, _: &ScrollController) -> Widget {
        self.child.clone()
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(BoxRenderSliver::new(self.child.clone()))
    }
}

/// A lazy variable-extent list sliver.
///
/// Use [`SliverFixedExtentList`] when every child has a known extent, or
/// [`SliverVariedExtentList`] when the extent comes from an index builder.
#[derive(Clone)]
pub struct SliverList {
    item_count: usize,
    builder: Rc<dyn Fn(usize) -> Widget>,
}

impl SliverList {
    #[must_use]
    pub fn builder<W>(item_count: usize, builder: impl Fn(usize) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            item_count,
            builder: Rc::new(move |i| builder(i).into()),
        }
    }
}

impl SliverList {
    /// Flutter's `SliverList(delegate:)`.
    #[must_use]
    pub fn delegate(delegate: impl SliverChildDelegate + 'static) -> Self {
        let (count, builder) = delegate_builder(Rc::new(delegate));
        Self::builder(count, builder)
    }
}

impl Sliver for SliverList {
    fn build(&self, controller: &ScrollController) -> Widget {
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, Axis::Vertical, false),
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let mut sliver = VariableExtentRenderSliver::new(
            MeasuredExtentIndex::new(self.item_count, DEFAULT_LAZY_ITEM_EXTENT),
            self.builder.clone(),
        );
        sliver.sample_scroll_extent = true;
        Box::new(sliver)
    }
}

/// A lazy grid sliver driven by a [`SliverGridDelegate`].
#[derive(Clone)]
pub struct SliverGrid {
    item_count: usize,
    delegate: SliverGridDelegate,
    builder: Rc<dyn Fn(usize) -> Widget>,
}

impl SliverGrid {
    #[must_use]
    pub fn builder<W>(
        item_count: usize,
        delegate: SliverGridDelegate,
        builder: impl Fn(usize) -> W + 'static,
    ) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            item_count,
            delegate,
            builder: Rc::new(move |i| builder(i).into()),
        }
    }
}

impl SliverGrid {
    /// Flutter's `SliverGrid(delegate:, gridDelegate:)`.
    #[must_use]
    pub fn delegate(
        delegate: impl SliverChildDelegate + 'static,
        grid_delegate: SliverGridDelegate,
    ) -> Self {
        let (count, builder) = delegate_builder(Rc::new(delegate));
        Self::builder(count, grid_delegate, builder)
    }
}

impl Sliver for SliverGrid {
    fn build(&self, controller: &ScrollController) -> Widget {
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, Axis::Vertical, false),
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(GridRenderSliver::new(
            self.item_count,
            self.delegate.clone(),
            self.builder.clone(),
            axis,
        ))
    }
}

/// Insets around a sliver child.
#[derive(TypedBuilder)]
pub struct SliverPadding {
    #[builder(setter(into))]
    padding: crate::EdgeInsetsGeometry,
    #[builder(default, setter(transform = |sliver: impl Sliver + 'static| {
        Some(Box::new(sliver) as Box<dyn Sliver>)
    }))]
    sliver: Option<Box<dyn Sliver>>,
}

impl SliverPadding {
    #[must_use]
    pub fn new(
        padding: impl Into<crate::EdgeInsetsGeometry>,
        sliver: impl Sliver + 'static,
    ) -> Self {
        Self {
            padding: padding.into(),
            sliver: Some(Box::new(sliver)),
        }
    }

    /// A sliver with only padding and no mounted child.
    #[must_use]
    pub const fn empty(padding: EdgeInsets) -> Self {
        Self {
            padding: crate::EdgeInsetsGeometry::Physical(padding),
            sliver: None,
        }
    }
}

impl Sliver for SliverPadding {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        let child = self.sliver.as_ref().map_or_else(
            || SizedBox::shrink().into(),
            |sliver| sliver.build_with_config(controller, axis, reverse),
        );
        match self.padding {
            crate::EdgeInsetsGeometry::Physical(padding) => Padding::new(padding, child).into(),
            padding => {
                crate::Builder::new(move |context| {
                    Padding::new(
                        padding
                            .resolve(context.text_direction().expect("validated Directionality")),
                        child.clone(),
                    )
                    .into()
                })
                .require_directionality(&crate::tree::DirectionRequirement {
                    widget: "SliverPadding",
                    reason: "resolving directional padding",
                })
                .into()
            }
        }
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(PaddingRenderSliver {
            inner: RefCell::new(self.sliver.as_ref().map_or_else(
                || Box::new(EmptyRenderSliver) as Box<dyn RenderSliver>,
                |sliver| sliver.create_render_sliver(controller, axis, reverse),
            )),
            padding: self.padding,
            safe_padding: None,
        })
    }
}

/// A pinned header sliver.
#[derive(TypedBuilder)]
pub struct SliverPersistentHeader {
    #[builder(setter(transform = |height: f32| height.max(0.0)))]
    height: f32,
    #[builder(setter(into))]
    child: Widget,
    #[builder(default = false)]
    pinned: bool,
}

impl SliverPersistentHeader {
    /// Flutter `SliverPersistentHeader(delegate: ...)` with variable extents:
    /// see [`SliverResizingHeader::with_delegate`].
    #[must_use]
    pub fn with_delegate<W>(
        min_extent: f32,
        max_extent: f32,
        build: impl Fn(f32, bool) -> W + 'static,
    ) -> SliverResizingHeader
    where
        W: Into<Widget>,
    {
        SliverResizingHeader::with_delegate(min_extent, max_extent, build)
    }

    #[must_use]
    pub fn new(height: f32, child: impl Into<Widget>) -> Self {
        Self {
            height: height.max(0.0),
            child: child.into(),
            pinned: false,
        }
    }

    #[must_use]
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    #[must_use]
    pub fn height(&self) -> f32 {
        self.height
    }
}

impl Sliver for SliverPersistentHeader {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        // Persistent headers have a stable sliver extent.  Keep the supplied
        // child as the retained subtree while constraining only its main-axis
        // extent; this makes `height()` affect real layout instead of being a
        // descriptor-only value.
        let child = if axis.is_horizontal() {
            SizedBox::new().width(self.height).child(self.child.clone())
        } else {
            SizedBox::new()
                .height(self.height)
                .child(self.child.clone())
        };
        Widget::persistent_header_with_config(
            controller.clone(),
            child.into(),
            axis,
            reverse,
            self.pinned,
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(HeaderRenderSliver::new(
            self.child.clone(),
            self.height,
            self.pinned,
        ))
    }
}

/// A framework-neutral app bar sliver.
#[derive(TypedBuilder)]
pub struct SliverAppBar {
    #[builder(default = 56.0, setter(transform = |height: f32| height.max(0.0)))]
    expanded_height: f32,
    #[builder(setter(into))]
    title: Widget,
    #[builder(default = true)]
    pinned: bool,
}

impl SliverAppBar {
    #[must_use]
    pub fn new(title: impl Into<Widget>) -> Self {
        Self {
            expanded_height: 56.0,
            title: title.into(),
            pinned: true,
        }
    }

    #[must_use]
    pub fn expanded_height(mut self, height: f32) -> Self {
        self.expanded_height = height.max(0.0);
        self
    }

    #[must_use]
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }
}

impl Sliver for SliverAppBar {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        SliverPersistentHeader::new(self.expanded_height, self.title.clone())
            .pinned(self.pinned)
            .build_with_config(controller, axis, reverse)
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(HeaderRenderSliver::new(
            self.title.clone(),
            self.expanded_height,
            self.pinned,
        ))
    }
}

/// A first-class CustomScrollView coordinating a sequence of slivers.
#[derive(TypedBuilder)]
pub struct CustomScrollView {
    #[builder(
        default,
        setter(transform = |slivers: impl IntoIterator<Item = Box<dyn Sliver>>| {
            slivers.into_iter().collect::<Vec<Box<dyn Sliver>>>()
        })
    )]
    slivers: Vec<Box<dyn Sliver>>,
    #[builder(default, setter(strip_option))]
    controller: Option<ScrollController>,
    /// Total indexed children in the semantic collection, when supplied.
    #[builder(default, setter(strip_option))]
    semantic_child_count: Option<usize>,
    #[builder(default = WidgetDefaults::DEFAULT.scroll_direction)]
    scroll_direction: Axis,
    #[builder(default = false)]
    reverse: bool,
    #[builder(default, setter(strip_option))]
    physics: Option<ScrollPhysics>,
    /// Overrides the default 50-pixel line and 80%-viewport page increments.
    #[builder(default, setter(transform = |calculator: impl Fn(ScrollIncrementDetails) -> f32 + 'static| {
        Some(ScrollIncrementCalculator::new(calculator))
    }))]
    increment_calculator: Option<ScrollIncrementCalculator>,
    #[builder(
        default = ScrollCacheExtent::pixels(WidgetDefaults::DEFAULT.sliver_cache_extent),
        setter(transform = |extent: f32| ScrollCacheExtent::pixels(extent.max(0.0)))
    )]
    cache_extent: ScrollCacheExtent,
    /// Flutter `clipBehavior`; defaults to `Clip.hardEdge`.
    #[builder(default = WidgetDefaults::DEFAULT.scroll_clip_behavior)]
    clip_behavior: Clip,
    /// Flutter `dragStartBehavior`; defaults to `DragStartBehavior.start`.
    #[builder(default)]
    drag_start_behavior: DragStartBehavior,
    /// Flutter `hitTestBehavior`; defaults to `HitTestBehavior.opaque`.
    #[builder(default = crate::gestures::HitTestBehavior::Opaque)]
    hit_test_behavior: crate::gestures::HitTestBehavior,
    /// Saves and restores the offset in the nearest `PageStorage`, as a
    /// Flutter scroll view keyed with a `PageStorageKey` does.
    #[builder(default, setter(strip_option))]
    page_storage_key: Option<crate::PageStorageKey>,
    /// Flutter `paintOrder`; defaults to [`SliverPaintOrder::FirstIsTop`].
    #[builder(default)]
    paint_order: SliverPaintOrder,
    /// Flutter `center`, by index: the sliver at scroll offset zero.
    #[builder(default)]
    center: usize,
    /// Flutter `anchor`: where offset zero sits, as a fraction of the
    /// viewport from its leading edge.
    #[builder(default)]
    anchor: f32,
}

impl Default for CustomScrollView {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl CustomScrollView {
    /// Creates a CustomScrollView with a list of slivers.
    #[must_use]
    pub fn new(slivers: impl IntoIterator<Item = Box<dyn Sliver>>) -> Self {
        Self {
            slivers: slivers.into_iter().collect(),
            controller: None,
            semantic_child_count: None,
            scroll_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
            increment_calculator: None,
            cache_extent: ScrollCacheExtent::pixels(WidgetDefaults::DEFAULT.sliver_cache_extent),
            paint_order: SliverPaintOrder::FirstIsTop,
            center: 0,
            anchor: 0.,
        }
    }

    /// Sets the scroll controller.
    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Sets the total indexed child count reported to assistive technology.
    #[must_use]
    pub fn semantic_child_count(mut self, count: usize) -> Self {
        self.semantic_child_count = Some(count);
        self
    }

    /// Sets scroll direction.
    #[must_use]
    pub fn scroll_direction(mut self, direction: Axis) -> Self {
        self.scroll_direction = direction;
        self
    }

    /// Sets whether the scroll view scrolls in reverse.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Sets scroll physics.
    #[must_use]
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    /// Supplies the actual pixel increment using the invoked scrollable's metrics.
    #[must_use]
    pub fn increment_calculator(
        mut self,
        calculator: impl Fn(ScrollIncrementDetails) -> f32 + 'static,
    ) -> Self {
        self.increment_calculator = Some(ScrollIncrementCalculator::new(calculator));
        self
    }


    /// Sets how the viewport clips its content (Flutter `clipBehavior`).
    /// `Clip::None` paints children that extend past the viewport.
    #[must_use]
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Sets where a scroll drag measures its start from (Flutter
    /// `dragStartBehavior`).
    #[must_use]
    pub fn drag_start_behavior(mut self, behavior: DragStartBehavior) -> Self {
        self.drag_start_behavior = behavior;
        self
    }

    /// Whether presses where no child is hit still reach the scrollable
    /// (Flutter `hitTestBehavior`, default opaque).
    #[must_use]
    pub fn hit_test_behavior(mut self, behavior: crate::gestures::HitTestBehavior) -> Self {
        self.hit_test_behavior = behavior;
        self
    }

    /// Keys this view's offset in the nearest `PageStorage` (Flutter
    /// `PageStorageKey`): with the controller's `keep_scroll_offset`, the
    /// offset survives the view being rebuilt or remounted.
    #[must_use]
    pub fn page_storage_key(mut self, key: crate::PageStorageKey) -> Self {
        self.page_storage_key = Some(key);
        self
    }

    /// Sets the amount of logical content laid out before and after the
    /// visible viewport. A nonzero cache keeps scrolling from synchronously
    /// constructing the next child at the edge.
    #[must_use]
    pub fn cache_extent(self, extent: f32) -> Self {
        self.scroll_cache_extent(ScrollCacheExtent::pixels(extent.max(0.)))
    }

    /// Sets the cache before and after the visible viewport as logical
    /// pixels or as a multiple of its current main-axis extent.
    #[must_use]
    pub fn scroll_cache_extent(mut self, extent: ScrollCacheExtent) -> Self {
        self.cache_extent = extent;
        self
    }

    /// Which sliver paints on top where slivers overlap (Flutter
    /// `CustomScrollView.paintOrder`); hit testing runs in the opposite
    /// order.
    #[must_use]
    pub fn paint_order(mut self, order: SliverPaintOrder) -> Self {
        self.paint_order = order;
        self
    }

    /// Flutter `CustomScrollView.center`, naming the sliver by its index:
    /// that sliver starts at scroll offset zero, the slivers after it follow
    /// it, and the slivers before it grow in reverse from its leading edge,
    /// at negative offsets. Defaults to the first sliver; an index past the
    /// end names the last.
    #[must_use]
    pub fn center(mut self, index: usize) -> Self {
        self.center = index;
        self
    }

    /// Flutter `CustomScrollView.anchor`: where scroll offset zero sits, as
    /// a fraction of the viewport from its leading edge, clamped to
    /// `0..=1`. With `0.5` the center sliver starts mid-viewport at offset
    /// zero. Defaults to `0`.
    #[must_use]
    pub fn anchor(mut self, anchor: f32) -> Self {
        self.anchor = anchor;
        self
    }

    #[must_use]
    pub fn is_reverse(&self) -> bool {
        self.reverse
    }

    #[must_use]
    pub fn get_scroll_direction(&self) -> Axis {
        self.scroll_direction
    }
}

impl From<CustomScrollView> for Widget {
    fn from(value: CustomScrollView) -> Self {
        value.into_viewport(true)
    }
}

impl CustomScrollView {
    fn into_viewport(mut self, ambient_horizontal_axis: bool) -> Widget {
        let implicit = self.controller.is_none() && self.page_storage_key.is_none();
        let controller = self
            .controller
            .get_or_insert_with(ScrollController::default)
            .clone();
        let key = self.page_storage_key.take();
        let increment_calculator = self.increment_calculator.take();
        let inherit = self
            .physics
            .is_none()
            .then_some(super::scroll_views::InheritedPhysics::Scroll);
        let mut viewport = custom_scroll_view_widget(self, ambient_horizontal_axis).with_scroll_increment_calculator(increment_calculator);
        if implicit && let WidgetKind::SliverViewport { config } = viewport.kind_mut() {
            Rc::make_mut(config).preserve_implicit_scroll_position = true;
        }
        keep_scroll_offset(super::inherit_physics(viewport, inherit), &controller, key)
    }
}
fn custom_scroll_view_widget(value: CustomScrollView, ambient_horizontal_axis: bool) -> Widget {
    let controller = value.controller.unwrap_or_default();
    let axis = value.scroll_direction;
    let reverse = value.reverse;
    let center = ViewportCenter {
        index: value.center,
        anchor: value.anchor,
    };
    let make_delegate =
        move |slivers: &[Box<dyn Sliver>], controller: &ScrollController, reverse| {
            let render_slivers = slivers
                .iter()
                .map(|sliver| sliver.create_render_sliver(controller, axis, reverse))
                .collect();
            Rc::new(SequenceViewportDelegate::centered(render_slivers, center))
        };
    let delegate = make_delegate(&value.slivers, &controller, reverse);
    let axis_resolution = if ambient_horizontal_axis && axis == Axis::Horizontal {
        let controller = controller.clone();
        let slivers = value.slivers;
        SliverAxisResolution::InheritedWithDelegate(Rc::new(
            geometry::AlternateSliverDelegate::new(move || {
                make_delegate(&slivers, &controller, !reverse)
            }),
        ))
    } else {
        SliverAxisResolution::Physical
    };
    let mut widget = Widget::sliver_viewport_with_delegate_options(
        controller,
        axis,
        reverse,
        value.physics.unwrap_or_default(),
        value.cache_extent.value(),
        false,
        value.clip_behavior,
        value.drag_start_behavior,
        value.hit_test_behavior,
        delegate,
    );
    if let WidgetKind::SliverViewport { config } = widget.kind_mut() {
        let config = Rc::make_mut(config);
        config.paint_order = value.paint_order;
        config.cache_extent = value.cache_extent;
        config.semantic_child_count = value.semantic_child_count;
        config.axis_resolution = axis_resolution;
    }
    widget
}

/// Underlying scroll gesture and viewport coordinator.
///
/// The axis lives with the built viewport, not here: the builder receives
/// only the controller, so a direction set on `Scrollable` could never
/// reach layout.
#[derive(Clone)]
pub struct Scrollable {
    controller: Option<ScrollController>,
    increment_calculator: Option<ScrollIncrementCalculator>,
    viewport_builder: Rc<dyn Fn(&ScrollController) -> Widget>,
}

impl Scrollable {
    #[must_use]
    pub fn new<W>(viewport_builder: impl Fn(&ScrollController) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            controller: None,
            increment_calculator: None,
            viewport_builder: Rc::new(move |c| viewport_builder(c).into()),
        }
    }

    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Controls action increments for the viewport returned by the builder.
    #[must_use]
    pub fn increment_calculator(
        mut self,
        calculator: impl Fn(ScrollIncrementDetails) -> f32 + 'static,
    ) -> Self {
        self.increment_calculator = Some(ScrollIncrementCalculator::new(calculator));
        self
    }

}

impl From<Scrollable> for Widget {
    fn from(value: Scrollable) -> Self {
        let controller = value.controller.unwrap_or_default();
        let viewport = (value.viewport_builder)(&controller);
        let Some(calculator) = value.increment_calculator else {
            return viewport;
        };
        if matches!(viewport.kind(), WidgetKind::Scroll { .. } | WidgetKind::SliverViewport { .. }) {
            viewport.with_scroll_increment_calculator(Some(calculator))
        } else {
            Widget::environment_scope(ScopedScrollIncrement { controller, calculator }, viewport)
        }
    }
}

/// Coordinates outer and inner scroll views with sliver headers.
#[derive(TypedBuilder)]
pub struct NestedScrollView {
    #[builder(default, setter(strip_option))]
    controller: Option<ScrollController>,
    #[builder(
        default,
        setter(transform = |slivers: impl IntoIterator<Item = Box<dyn Sliver>>| {
            slivers.into_iter().collect::<Vec<Box<dyn Sliver>>>()
        })
    )]
    header_slivers: Vec<Box<dyn Sliver>>,
    #[builder(setter(into))]
    body: Widget,
    /// Flutter `NestedScrollView.floatHeaderSlivers`.
    #[builder(default)]
    float_header_slivers: bool,
}

impl NestedScrollView {
    #[must_use]
    pub fn new(
        header_slivers: impl IntoIterator<Item = Box<dyn Sliver>>,
        body: impl Into<Widget>,
    ) -> Self {
        Self {
            controller: None,
            header_slivers: header_slivers.into_iter().collect(),
            body: body.into(),
            float_header_slivers: false,
        }
    }

    /// The outer (header) position. The body's scroll view is the inner
    /// position; drags and flings are coordinated between the two as
    /// Flutter's `_NestedScrollCoordinator` does.
    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Flutter `floatHeaderSlivers`: a drag back moves the header before the
    /// body returns to its start.
    #[must_use]
    pub fn float_header_slivers(mut self, float: bool) -> Self {
        self.float_header_slivers = float;
        self
    }
}

impl From<NestedScrollView> for Widget {
    fn from(value: NestedScrollView) -> Self {
        let controller = value.controller.unwrap_or_default();
        // The controller is the outer position; the body's scroll view joins
        // as the inner position when a press over it first reaches the
        // coordinator (`ScrollController::bind_drag_chain`).
        let coordinator = controller.nested_outer_coordinator().unwrap_or_else(|| {
            incular_scroll::NestedScrollViewCoordinator::new(value.float_header_slivers)
        });
        coordinator.set_float_header_slivers(value.float_header_slivers);
        coordinator.attach_outer(&controller);
        let mut slivers = value.header_slivers;
        // Flutter `NestedScrollView._buildSlivers`: the body fills the
        // remaining viewport as a scroll body.
        slivers.push(Box::new(SliverFillRemaining::new(value.body)) as Box<dyn Sliver>);
        CustomScrollView::new(slivers).controller(controller).into()
    }
}

/// A viewport bounding visible slivers.
#[derive(TypedBuilder)]
pub struct Viewport {
    #[builder(
        default,
        setter(transform = |slivers: impl IntoIterator<Item = Box<dyn Sliver>>| {
            slivers.into_iter().collect::<Vec<Box<dyn Sliver>>>()
        })
    )]
    slivers: Vec<Box<dyn Sliver>>,
    #[builder(default, setter(strip_option))]
    controller: Option<ScrollController>,
    #[builder(default = ScrollCacheExtent::pixels(WidgetDefaults::DEFAULT.sliver_cache_extent))]
    scroll_cache_extent: ScrollCacheExtent,
    #[builder(default = WidgetDefaults::DEFAULT.scroll_direction)]
    axis_direction: Axis,
    #[builder(default)]
    reverse: bool,
    /// Flutter `Viewport.paintOrder`; defaults to `firstIsTop`.
    #[builder(default)]
    paint_order: SliverPaintOrder,
    /// Flutter `Viewport.center`, by index.
    #[builder(default)]
    center: usize,
    /// Flutter `Viewport.anchor`.
    #[builder(default)]
    anchor: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Viewport {
    /// The framework tag marking the immediate semantic children of a sliver
    /// viewport. Transparent layout wrappers carry it to the next node.
    #[must_use]
    pub fn use_two_pane_semantics_tag() -> &'static incular_semantics::SemanticsTag {
        static TAG: std::sync::OnceLock<incular_semantics::SemanticsTag> =
            std::sync::OnceLock::new();
        TAG.get_or_init(|| incular_semantics::SemanticsTag::new("RenderViewport.twoPane"))
    }

    /// The framework tag for immediate semantic children that stay outside
    /// the scrolling pane (for example a pinned header at the viewport edge).
    #[must_use]
    pub fn exclude_from_scrolling_tag() -> &'static incular_semantics::SemanticsTag {
        static TAG: std::sync::OnceLock<incular_semantics::SemanticsTag> =
            std::sync::OnceLock::new();
        TAG.get_or_init(|| incular_semantics::SemanticsTag::new("RenderViewport.excludeFromScrolling"))
    }

    #[must_use]
    pub fn new(slivers: impl IntoIterator<Item = Box<dyn Sliver>>) -> Self {
        Self {
            slivers: slivers.into_iter().collect(),
            controller: None,
            scroll_cache_extent: ScrollCacheExtent::pixels(WidgetDefaults::DEFAULT.sliver_cache_extent),
            axis_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            paint_order: SliverPaintOrder::FirstIsTop,
            center: 0,
            anchor: 0.,
        }
    }

    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Sets the viewport's offscreen cache and semantic reach, in
    /// logical pixels or multiples of the viewport's main-axis extent.
    #[must_use]
    pub fn scroll_cache_extent(mut self, extent: ScrollCacheExtent) -> Self {
        self.scroll_cache_extent = extent;
        self
    }

    #[must_use]
    pub fn axis_direction(mut self, axis: Axis) -> Self {
        self.axis_direction = axis;
        self
    }

    /// Lays slivers from the trailing edge: left for a horizontal
    /// viewport or up for a vertical viewport.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Which sliver paints on top where slivers overlap (Flutter
    /// `Viewport.paintOrder`).
    #[must_use]
    pub fn paint_order(mut self, order: SliverPaintOrder) -> Self {
        self.paint_order = order;
        self
    }

    /// Flutter `Viewport.center`, naming the sliver by its index: see
    /// [`CustomScrollView::center`].
    #[must_use]
    pub fn center(mut self, index: usize) -> Self {
        self.center = index;
        self
    }

    /// Flutter `Viewport.anchor`: see [`CustomScrollView::anchor`].
    #[must_use]
    pub fn anchor(mut self, anchor: f32) -> Self {
        self.anchor = anchor;
        self
    }

    #[must_use]
    pub fn get_axis_direction(&self) -> Axis {
        self.axis_direction
    }
}

impl From<Viewport> for Widget {
    fn from(value: Viewport) -> Self {
        CustomScrollView::new(value.slivers)
            .scroll_direction(value.axis_direction)
            .reverse(value.reverse)
            .controller(value.controller.unwrap_or_default())
            .scroll_cache_extent(value.scroll_cache_extent)
            .paint_order(value.paint_order)
            .center(value.center)
            .anchor(value.anchor)
            // RenderViewport itself is not the opaque Scrollable surface.
            // Space between sliver children exposes the underlying hit path.
            .hit_test_behavior(crate::gestures::HitTestBehavior::DeferToChild)
            .into_viewport(false)
    }
}

/// A viewport that sizes itself to its slivers along the main axis (Flutter
/// `ShrinkWrappingViewport`). It has no center: content starts at offset zero.
#[derive(TypedBuilder)]
pub struct ShrinkWrappingViewport {
    #[builder(
        default,
        setter(transform = |slivers: impl IntoIterator<Item = Box<dyn Sliver>>| {
            slivers.into_iter().collect::<Vec<Box<dyn Sliver>>>()
        })
    )]
    slivers: Vec<Box<dyn Sliver>>,
    #[builder(default, setter(strip_option))]
    controller: Option<ScrollController>,
    #[builder(default = WidgetDefaults::DEFAULT.scroll_direction)]
    axis_direction: Axis,
    #[builder(default)]
    reverse: bool,
}

impl Default for ShrinkWrappingViewport {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ShrinkWrappingViewport {
    #[must_use]
    pub fn new(slivers: impl IntoIterator<Item = Box<dyn Sliver>>) -> Self {
        Self {
            slivers: slivers.into_iter().collect(),
            controller: None,
            axis_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
        }
    }

    /// The position this viewport shows (Flutter `offset`).
    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// The main axis (Flutter `axisDirection`'s axis).
    #[must_use]
    pub fn axis_direction(mut self, axis: Axis) -> Self {
        self.axis_direction = axis;
        self
    }

    /// Whether offset zero is at the far end of the axis (Flutter
    /// `AxisDirection.up` or `left`).
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
}

impl From<ShrinkWrappingViewport> for Widget {
    fn from(value: ShrinkWrappingViewport) -> Self {
        let controller = value.controller.unwrap_or_default();
        let (axis, reverse) = (value.axis_direction, value.reverse);
        let render_slivers = value
            .slivers
            .iter()
            .map(|sliver| sliver.create_render_sliver(&controller, axis, reverse))
            .collect();
        Widget::sliver_viewport_with_delegate_options(
            controller,
            axis,
            reverse,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            true,
            WidgetDefaults::DEFAULT.scroll_clip_behavior,
            DragStartBehavior::Start,
            crate::gestures::HitTestBehavior::Opaque,
            Rc::new(SequenceViewportDelegate::new(render_slivers)),
        )
    }
}

/// Places every child sequentially along an unbounded main axis, stretching
/// each child to the available cross-axis extent.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ListBody {
    #[builder(default = Axis::Vertical)]
    main_axis: Axis,
    #[builder(default)]
    reverse: bool,
    #[builder(default, setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
        children.into_iter().map(Into::into).collect::<Vec<Widget>>()
    }))]
    children: Vec<Widget>,
}

impl Default for ListBody {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ListBody {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            main_axis: Axis::Vertical,
            reverse: false,
            children: children.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub fn main_axis(mut self, axis: Axis) -> Self {
        self.main_axis = axis;
        self
    }

    /// Flips the placement direction without changing the order of the children.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
}

impl From<ListBody> for Widget {
    fn from(value: ListBody) -> Self {
        Widget::from_kind(WidgetKind::ListBody {
            axis: value.main_axis,
            reverse: value.reverse,
            children: value.children,
        })
    }
}

/// Sliver list with fixed item extents.
pub struct SliverFixedExtentList {
    item_count: usize,
    item_extent: f32,
    builder: Rc<dyn Fn(usize) -> Widget>,
}

impl SliverFixedExtentList {
    #[must_use]
    pub fn new<W>(
        item_count: usize,
        item_extent: f32,
        builder: impl Fn(usize) -> W + 'static,
    ) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            item_count,
            item_extent: finite_non_negative(item_extent),
            builder: Rc::new(move |i| builder(i).into()),
        }
    }
}

impl Sliver for SliverFixedExtentList {
    fn build(&self, controller: &ScrollController) -> Widget {
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, Axis::Vertical, false),
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(FixedExtentRenderSliver::new(
            self.item_count,
            self.item_extent,
            self.builder.clone(),
        ))
    }
}

/// Sliver list with variable item extents.
pub struct SliverVariedExtentList {
    item_count: usize,
    extent_builder: Rc<dyn Fn(usize) -> f32>,
    item_builder: Rc<dyn Fn(usize) -> Widget>,
}

impl SliverVariedExtentList {
    #[must_use]
    pub fn new<W>(
        item_count: usize,
        extent_builder: impl Fn(usize) -> f32 + 'static,
        item_builder: impl Fn(usize) -> W + 'static,
    ) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            item_count,
            extent_builder: Rc::new(extent_builder),
            item_builder: Rc::new(move |i| item_builder(i).into()),
        }
    }

    #[must_use]
    pub fn extent_builder(&self) -> &Rc<dyn Fn(usize) -> f32> {
        &self.extent_builder
    }
}

impl Sliver for SliverVariedExtentList {
    fn build(&self, controller: &ScrollController) -> Widget {
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, Axis::Vertical, false),
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let index = MeasuredExtentIndex::with_estimates(
            self.item_count,
            DEFAULT_LAZY_ITEM_EXTENT,
            |index| (self.extent_builder)(index),
        );
        let mut sliver = VariableExtentRenderSliver::new(index, self.item_builder.clone());
        sliver.tight_main_extent = true;
        Box::new(sliver)
    }
}

/// Sliver list taking extent from a prototype widget.
pub struct SliverPrototypeExtentList {
    item_count: usize,
    prototype_item: Widget,
    builder: Rc<dyn Fn(usize) -> Widget>,
}

impl SliverPrototypeExtentList {
    #[must_use]
    pub fn new<W>(
        item_count: usize,
        prototype_item: impl Into<Widget>,
        builder: impl Fn(usize) -> W + 'static,
    ) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            item_count,
            prototype_item: prototype_item.into(),
            builder: Rc::new(move |i| builder(i).into()),
        }
    }

    #[must_use]
    pub fn prototype_item(&self) -> &Widget {
        &self.prototype_item
    }
}

impl Sliver for SliverPrototypeExtentList {
    fn build(&self, controller: &ScrollController) -> Widget {
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            self.create_render_sliver(controller, Axis::Vertical, false),
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        // Flutter's prototype sliver lays out one prototype and forces every
        // child to that main-axis extent. Use explicit widget dimensions when
        // available; constraint-dependent prototypes fall back to the normal
        // warm-up estimate and are corrected by the retained child pass.
        let prototype_extent = widget_main_extent_hint(&self.prototype_item, axis)
            .unwrap_or(DEFAULT_LAZY_ITEM_EXTENT)
            .max(1.);
        Box::new(FixedExtentRenderSliver::new(
            self.item_count,
            prototype_extent,
            self.builder.clone(),
        ))
    }
}

/// Sliver filling remaining viewport space.
///
/// With `has_scroll_body(false)` and [`fill_overscroll`](Self::fill_overscroll)
/// enabled, the child also fills the leading or trailing overscroll area of
/// bouncing physics, as Flutter's `SliverFillRemaining(fillOverscroll: true)`.
#[derive(TypedBuilder)]
pub struct SliverFillRemaining {
    #[builder(setter(into))]
    child: Widget,
    #[builder(default = true)]
    has_scroll_body: bool,
    #[builder(default)]
    fill_overscroll: bool,
}

impl SliverFillRemaining {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            has_scroll_body: true,
            fill_overscroll: false,
        }
    }

    #[must_use]
    pub fn has_scroll_body(mut self, has_scroll_body: bool) -> Self {
        self.has_scroll_body = has_scroll_body;
        self
    }

    /// Lets a non-scrolling child extend into overscroll (Flutter's
    /// `fillOverscroll`). Has no effect when `has_scroll_body` is true.
    #[must_use]
    pub fn fill_overscroll(mut self, fill_overscroll: bool) -> Self {
        self.fill_overscroll = fill_overscroll;
        self
    }

    #[must_use]
    pub fn is_scroll_body(&self) -> bool {
        self.has_scroll_body
    }
}

impl Sliver for SliverFillRemaining {
    fn build(&self, _controller: &ScrollController) -> Widget {
        self.child.clone()
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(FillRemainingRenderSliver {
            child: self.child.clone(),
            has_scroll_body: self.has_scroll_body,
            fill_overscroll: self.fill_overscroll,
        })
    }
}

/// Sliver with children each filling the entire viewport.
#[derive(TypedBuilder)]
#[builder(builder_method(name = typed_builder))]
pub struct SliverFillViewport {
    #[builder(default, setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
        Rc::new(children.into_iter().map(Into::into).collect::<Vec<Widget>>())
    }))]
    children: Rc<Vec<Widget>>,
    #[builder(default, setter(skip))]
    builder: Option<Rc<dyn Fn(usize) -> Widget>>,
    #[builder(default, setter(skip))]
    item_count: Option<usize>,
    #[builder(default, setter(skip))]
    find_child_index: Option<FindChildIndex>,
    #[builder(default = 1.0, setter(transform = |fraction: f32| fraction.max(0.01)))]
    viewport_fraction: f32,
    #[builder(default = WidgetDefaults::DEFAULT.sliver_fill_viewport_extent, setter(transform = |extent: f32| extent.max(1.0)))]
    fallback_extent: f32,
    /// Flutter `padEnds`: insets the first and last children by half the
    /// space `viewport_fraction` leaves unused. Defaults to true.
    #[builder(default = true)]
    pad_ends: bool,
    /// Includes retained cached pages in accessibility. Defaults to true.
    #[builder(default = true)]
    allow_implicit_scrolling: bool,
}

impl Default for SliverFillViewport {
    fn default() -> Self {
        Self::typed_builder().build()
    }
}

impl SliverFillViewport {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: Rc::new(children.into_iter().map(Into::into).collect()),
            builder: None,
            item_count: None,
            find_child_index: None,
            viewport_fraction: 1.0,
            fallback_extent: WidgetDefaults::DEFAULT.sliver_fill_viewport_extent,
            pad_ends: true,
            allow_implicit_scrolling: true,
        }
    }

    /// Creates a lazy viewport-filling sliver from an indexed child builder.
    #[must_use]
    pub fn builder<W>(item_count: usize, builder: impl Fn(usize) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            children: Rc::default(),
            builder: Some(Rc::new(move |index| builder(index).into())),
            item_count: Some(item_count),
            find_child_index: None,
            viewport_fraction: 1.0,
            fallback_extent: WidgetDefaults::DEFAULT.sliver_fill_viewport_extent,
            pad_ends: true,
            allow_implicit_scrolling: true,
        }
    }

    pub(super) fn with_child_index_lookup(mut self, callback: Option<FindChildIndex>) -> Self {
        if self.builder.is_some() {
            self.find_child_index = callback;
        }
        self
    }

    #[must_use]
    pub fn viewport_fraction(mut self, fraction: f32) -> Self {
        self.viewport_fraction = fraction.max(0.01);
        self
    }

    /// Sets the extent used when the sliver receives unbounded viewport
    /// constraints. Bounded viewports always derive the page extent from the
    /// viewport itself, matching Flutter's `SliverFillViewport`.
    #[must_use]
    pub fn fallback_extent(mut self, extent: f32) -> Self {
        self.fallback_extent = extent.max(1.0);
        self
    }

    #[must_use]
    pub fn viewport_fraction_value(&self) -> f32 {
        self.viewport_fraction
    }

    /// Expose cached offscreen pages to accessibility when true (default).
    /// When false, only pages overlapping the sliver's visible interval
    /// contribute semantic children; layout and caching are unchanged.
    #[must_use]
    pub fn allow_implicit_scrolling(mut self, allow: bool) -> Self {
        self.allow_implicit_scrolling = allow;
        self
    }

    /// Sets Flutter's `padEnds` (default true).
    #[must_use]
    pub fn pad_ends(mut self, pad_ends: bool) -> Self {
        self.pad_ends = pad_ends;
        self
    }
}

impl Sliver for SliverFillViewport {
    fn build(&self, controller: &ScrollController) -> Widget {
        let render = self.create_render_sliver(controller, Axis::Vertical, false);
        single_sliver_viewport(
            controller.clone(),
            Axis::Vertical,
            false,
            ScrollPhysics::default(),
            DEFAULT_SLIVER_CACHE_EXTENT,
            Clip::HardEdge,
            render,
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let item_count = self.item_count.unwrap_or(self.children.len());
        Box::new(ViewportExtentRenderSliver {
            item_count,
            children: (!self.children.is_empty()).then(|| Rc::clone(&self.children)),
            builder: self.builder.clone(),
            find_child_index: self.find_child_index.clone(),
            widgets: HashMap::new(),
            viewport_fraction: self.viewport_fraction,
            fallback_extent: self.fallback_extent,
            pad_ends: self.pad_ends,
            allow_implicit_scrolling: self.allow_implicit_scrolling,
        })
    }
}

/// A sliver builder that receives the current sliver constraints during
/// layout, matching Flutter's `SliverLayoutBuilder` contract.
pub struct SliverLayoutBuilder {
    builder: SliverLayoutBuilderFn,
}

impl SliverLayoutBuilder {
    #[must_use]
    pub fn new<W>(builder: impl Fn(SliverConstraints) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            builder: Rc::new(move |constraints| builder(constraints).into()),
        }
    }
}

impl Sliver for SliverLayoutBuilder {
    fn build(&self, _controller: &ScrollController) -> Widget {
        // The callback belongs to the render-sliver layout phase. Calling it
        // here would fabricate constraints and run application code twice:
        // once while constructing the widget description and again when the
        // viewport has real geometry. This placeholder is replaced by
        // `LayoutBuilderRenderSliver::perform_layout`.
        SizedBox::shrink().into()
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(LayoutBuilderRenderSliver {
            builder: self.builder.clone(),
            child: SizedBox::shrink().into(),
            extent: Cell::new(DEFAULT_LAZY_ITEM_EXTENT),
            last_constraints: None,
            revision: 0,
        })
    }
}

/// Groups multiple slivers along the main axis.
#[derive(TypedBuilder)]
pub struct SliverMainAxisGroup {
    #[builder(
        default,
        setter(transform = |slivers: impl IntoIterator<Item = Box<dyn Sliver>>| {
            slivers.into_iter().collect::<Vec<Box<dyn Sliver>>>()
        })
    )]
    slivers: Vec<Box<dyn Sliver>>,
}

impl Default for SliverMainAxisGroup {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl SliverMainAxisGroup {
    #[must_use]
    pub fn new(slivers: impl IntoIterator<Item = Box<dyn Sliver>>) -> Self {
        Self {
            slivers: slivers.into_iter().collect(),
        }
    }
}

impl Sliver for SliverMainAxisGroup {
    fn build(&self, controller: &ScrollController) -> Widget {
        let built = self
            .slivers
            .iter()
            .map(|s| s.build(controller))
            .collect::<Vec<_>>();
        Column::new(built).into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(SequenceRenderSliver::new(
            self.slivers
                .iter()
                .map(|sliver| sliver.create_render_sliver(controller, axis, reverse))
                .collect(),
        ))
    }
}

/// Groups multiple slivers across the cross axis.
#[derive(TypedBuilder)]
pub struct SliverCrossAxisGroup {
    #[builder(
        default,
        setter(transform = |slivers: impl IntoIterator<Item = Box<dyn Sliver>>| {
            slivers.into_iter().collect::<Vec<Box<dyn Sliver>>>()
        })
    )]
    slivers: Vec<Box<dyn Sliver>>,
}

impl Default for SliverCrossAxisGroup {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl SliverCrossAxisGroup {
    #[must_use]
    pub fn new(slivers: impl IntoIterator<Item = Box<dyn Sliver>>) -> Self {
        Self {
            slivers: slivers.into_iter().collect(),
        }
    }
}

impl Sliver for SliverCrossAxisGroup {
    fn build(&self, controller: &ScrollController) -> Widget {
        let built = self
            .slivers
            .iter()
            .map(|s| s.build(controller))
            .collect::<Vec<_>>();
        Row::new(built).into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(CrossAxisGroupRenderSliver::new(
            self.slivers
                .iter()
                .map(|sliver| {
                    (
                        sliver.create_render_sliver(controller, axis, reverse),
                        sliver.cross_axis_flex().unwrap_or(1),
                    )
                })
                .collect(),
        ))
    }
}

/// Lays out several slivers against the same main-axis scroll position while
/// dividing the cross axis into flex lanes. Each lane has independent sliver
/// geometry, but the group consumes the maximum lane scroll extent.
pub(super) struct CrossAxisGroupRenderSliver {
    children: Vec<(Box<dyn RenderSliver>, usize)>,
}

impl CrossAxisGroupRenderSliver {
    fn new(children: Vec<(Box<dyn RenderSliver>, usize)>) -> Self {
        Self { children }
    }

    pub(super) fn adopt_compatible_state(
        &mut self,
        previous: &mut Self,
        probes: &mut u64,
        preserve_anchor: bool,
    ) {
        for ((fresh, _), (retained, _)) in self.children.iter_mut().zip(&mut previous.children) {
            super::layout::transfer_retained_sliver_state(
                &mut **fresh,
                &mut **retained,
                probes,
                preserve_anchor,
            );
        }
    }
}

impl RenderSliver for CrossAxisGroupRenderSliver {
    fn reveal_child(
        &mut self,
        child: SliverChildId,
        rect: Rect,
        options: &ShowOnScreen,
    ) -> Option<Rect> {
        let (sliver, _) = self.children.get_mut(child.scope())?;
        sliver.reveal_child(child.local(), rect, options)
    }

    fn set_child_focus_keep_alive(
        &mut self,
        child: SliverChildId,
        grid_tile: Option<usize>,
        keep_alive: bool,
    ) -> bool {
        // The retained viewport addresses a flattened row. Restore the
        // lane-local ID before its lazy delegate handles editor focus.
        self.children
            .get_mut(child.scope())
            .is_some_and(|(sliver, _)| {
                sliver.set_child_focus_keep_alive(child.local(), grid_tile, keep_alive)
            })
    }

    fn scroll_layout_dependency(&self) -> SliverScrollDependency {
        self.children
            .iter()
            .map(|(child, _)| child.scroll_layout_dependency())
            .max()
            .unwrap_or(SliverScrollDependency::CacheWindow)
    }

    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        // Flutter lays out fixed lanes first, subtracting the width that
        // each actually reports. Only remaining space is divided by flex.
        let mut fixed = Vec::new();
        let mut total_flex = 0usize;
        let mut remaining = constraints.cross_axis_extent;
        for (sliver, flex) in &mut self.children {
            if *flex != 0 {
                total_flex = total_flex.saturating_add(*flex);
                continue;
            }
            let layout = sliver.perform_layout(SliverConstraints {
                cross_axis_extent: remaining,
                ..constraints
            });
            let reported = sliver.reported_cross_axis_extent();
            debug_assert!(reported.is_some(), "a zero-flex sliver must report its cross extent");
            let extent = reported.unwrap_or(remaining).clamp(0., remaining);
            remaining -= extent;
            fixed.push((layout, extent));
        }
        let extent_per_flex = remaining / total_flex.max(1) as f32;
        let mut fixed = fixed.into_iter();
        let mut cross_offset = 0.;
        let mut geometry = SliverGeometry::ZERO;
        let mut absorbed_overlap = 0.;
        let mut lanes = Vec::with_capacity(self.children.len());
        for (lane, (sliver, flex)) in self.children.iter_mut().enumerate() {
            let (layout, lane_extent) = if *flex == 0 {
                fixed.next().expect("every fixed lane was already laid out")
            } else {
                let extent = extent_per_flex * *flex as f32;
                let layout = sliver.perform_layout(SliverConstraints {
                    cross_axis_extent: extent,
                    ..constraints
                });
                let reported = sliver.reported_cross_axis_extent().unwrap_or(extent);
                (layout, reported.clamp(0., extent))
            };
            let lane_geometry = layout.geometry.normalized();
            if lane_geometry.scroll_offset_correction.is_some() {
                return SliverLayout::empty(lane_geometry);
            }
            if lane_geometry.scroll_extent > geometry.scroll_extent {
                geometry = lane_geometry;
                absorbed_overlap = layout.absorbed_overlap;
            }
            lanes.push((lane, cross_offset, lane_geometry, layout.children));
            cross_offset += lane_extent;
        }

        // Pin independently in each lane, then push its paint back inside
        // the group's trailing edge. A viewport-wide pin stack would make
        // neighboring lanes obstruct each other or pin over the next sliver.
        let remaining_main = geometry.scroll_extent - constraints.scroll_offset;
        let reverse =
            constraints.reverse ^ (constraints.growth_direction == SliverGrowthDirection::Reverse);
        let mut children = Vec::with_capacity(lanes.iter().map(|lane| lane.3.len()).sum());
        for (lane, cross, lane_geometry, placements) in lanes {
            let correction = (lane_geometry.paint_extent - remaining_main).max(0.);
            // Flutter sets the lane's physical paint offset even when the
            // main axis runs backwards. Translate from the group's physical
            // paint origin into the viewport's logical scroll coordinates.
            let main_adjustment = if reverse {
                geometry.paint_extent - lane_geometry.paint_extent + correction
            } else {
                -correction
            };
            for mut child in placements {
                child.id = SliverChildId::scoped(lane, child.id);
                child.cross_offset += cross;
                if child.placement == SliverChildPlacement::Pinned {
                    child.offset = child.offset.max(constraints.scroll_offset);
                    child.placement = SliverChildPlacement::Floating;
                }
                child.offset += main_adjustment;
                children.push(child);
            }
        }
        SliverLayout {
            geometry,
            children,
            absorbed_overlap,
        }
    }

    fn set_child_extent(&mut self, child: SliverChildId, extent: f32) -> bool {
        let lane = child.scope();
        self.children
            .get_mut(lane)
            .is_some_and(|(sliver, _)| sliver.set_child_extent(child.local(), extent))
    }

    fn revision(&self) -> u64 {
        self.children
            .iter()
            .map(|(sliver, _)| sliver.revision())
            .fold(0, u64::wrapping_add)
    }

    fn tick(&mut self, now: Instant) -> bool {
        // Each lane advances independently in the same frame.
        self.children
            .iter_mut()
            .fold(false, |changed, (sliver, _)| sliver.tick(now) | changed)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn is_animating(&self) -> bool {
        self.children
            .iter()
            .any(|(sliver, _)| sliver.is_animating())
    }

    fn set_reduced_motion(&mut self, reduced: bool) {
        for (sliver, _) in &mut self.children {
            sliver.set_reduced_motion(reduced);
        }
    }
}

/// Expands a sliver across cross-axis group space.
#[derive(TypedBuilder)]
pub struct SliverCrossAxisExpanded {
    #[builder(default = 1, setter(transform = |flex: usize| flex.max(1)))]
    flex: usize,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
}

impl SliverCrossAxisExpanded {
    #[must_use]
    pub fn new(flex: usize, sliver: impl Sliver + 'static) -> Self {
        Self {
            flex: flex.max(1),
            sliver: Box::new(sliver),
        }
    }
}

impl Sliver for SliverCrossAxisExpanded {
    fn build(&self, controller: &ScrollController) -> Widget {
        crate::layout::Expanded::new(self.sliver.build(controller))
            .flex(self.flex as u32)
            .into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        self.sliver.create_render_sliver(controller, axis, reverse)
    }

    fn cross_axis_flex(&self) -> Option<usize> {
        Some(self.flex)
    }
}

/// Constrains the cross-axis dimension of a sliver.
#[derive(TypedBuilder)]
pub struct SliverConstrainedCrossAxis {
    #[builder(setter(transform = |extent: f32| extent.max(0.0)))]
    max_extent: f32,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
}

impl SliverConstrainedCrossAxis {
    #[must_use]
    pub fn new(max_extent: f32, sliver: impl Sliver + 'static) -> Self {
        Self {
            max_extent: max_extent.max(0.0),
            sliver: Box::new(sliver),
        }
    }
}

impl Sliver for SliverConstrainedCrossAxis {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(&self, controller: &ScrollController, axis: Axis, reverse: bool) -> Widget {
        let bounds = if axis.is_vertical() {
            Size::new(self.max_extent, f32::INFINITY)
        } else {
            Size::new(f32::INFINITY, self.max_extent)
        };
        crate::layout::ConstrainedBox::new(
            Constraints::loose(bounds),
            self.sliver.build_with_config(controller, axis, reverse),
        ).into()
    }

    fn cross_axis_flex(&self) -> Option<usize> {
        Some(0)
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(super::layout::ConstrainedCrossAxisRenderSliver::new(
            self.sliver.create_render_sliver(controller, axis, reverse), self.max_extent,
        ))
    }
}

/// Paints one decoration behind or in front of an entire sliver.
#[derive(TypedBuilder)]
pub struct DecoratedSliver {
    #[builder(default)]
    decoration: incular_rendering::Decoration,
    #[builder(default, setter(strip_option))]
    box_decoration: Option<crate::BoxDecoration>,
    #[builder(default, setter(strip_option))]
    painter_factory: Option<SliverBoxPainterFactory>,
    #[builder(default)]
    position: crate::DecorationPosition,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
}

impl DecoratedSliver {
    #[must_use]
    pub fn new(decoration: incular_rendering::Decoration, sliver: impl Sliver + 'static) -> Self {
        Self {
            decoration,
            box_decoration: None,
            painter_factory: None,
            position: crate::DecorationPosition::Background,
            sliver: Box::new(sliver),
        }
    }

    /// Uses the same BoxDecoration painter as a mounted DecoratedBox.
    #[must_use]
    pub fn new_box(decoration: crate::BoxDecoration, sliver: impl Sliver + 'static) -> Self {
        Self {
            box_decoration: Some(decoration),
            ..Self::new(incular_rendering::Decoration::default(), sliver)
        }
    }

    /// Uses a custom box painter retained by the viewport's render sliver.
    #[must_use]
    pub fn new_with_painter(
        factory: SliverBoxPainterFactory,
        sliver: impl Sliver + 'static,
    ) -> Self {
        Self {
            painter_factory: Some(factory),
            ..Self::new(incular_rendering::Decoration::default(), sliver)
        }
    }

    #[must_use]
    pub fn position(mut self, position: crate::DecorationPosition) -> Self {
        self.position = position;
        self
    }

    /// The rendering data from the low-level constructor.
    #[must_use]
    pub fn decoration(&self) -> &incular_rendering::Decoration {
        &self.decoration
    }

    #[must_use]
    pub fn box_decoration(&self) -> Option<&crate::BoxDecoration> {
        self.box_decoration.as_ref()
    }

    fn decorator(&self) -> Rc<dyn Fn(Widget) -> Widget> {
        if let Some(decoration) = &self.box_decoration {
            let decoration = decoration.clone();
            let position = self.position;
            return Rc::new(move |child| {
                crate::painting_effects::decorated(decoration.clone(), position, child)
            });
        }
        let decoration = self.decoration.clone();
        Rc::new(move |child| {
            Widget::decorated(
                None, decoration.background.clone(), decoration.border,
                decoration.border_radius, child,
            )
        })
    }
}

impl Sliver for DecoratedSliver {
    fn build(&self, controller: &ScrollController) -> Widget {
        let child = self.sliver.build(controller);
        let decorate = self.decorator();
        if self.box_decoration.is_none() && self.position == crate::DecorationPosition::Foreground {
            let decoration = crate::IgnorePointer::new(crate::ExcludeSemantics::new(
                decorate(crate::SizedBox::shrink().into()),
            ));
            crate::Stack::new([child, crate::Positioned::fill(decoration).into()]).into()
        } else {
            decorate(child)
        }
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let inner = self.sliver.create_render_sliver(controller, axis, reverse);
        Box::new(super::sliver_decoration::DecoratedRenderSliver::new(
            inner, self.decorator(), self.position, self.painter_factory.clone(),
        ))
    }
}

/// Sliver opacity wrapper.
#[derive(TypedBuilder)]
pub struct SliverOpacity {
    #[builder(setter(transform = |opacity: f32| opacity.clamp(0.0, 1.0)))]
    opacity: f32,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
    #[builder(default)]
    always_include_semantics: bool,
}

impl SliverOpacity {
    #[must_use]
    pub fn new(opacity: f32, sliver: impl Sliver + 'static) -> Self {
        Self {
            opacity: opacity.clamp(0.0, 1.0),
            sliver: Box::new(sliver),
            always_include_semantics: false,
        }
    }

    /// Flutter's `alwaysIncludeSemantics`: keep the sliver's semantics while
    /// the opacity is zero.
    #[must_use]
    pub fn always_include_semantics(mut self, include: bool) -> Self {
        self.always_include_semantics = include;
        self
    }
}

impl Sliver for SliverOpacity {
    fn build(&self, controller: &ScrollController) -> Widget {
        crate::Opacity::new(self.opacity, self.sliver.build(controller))
            .always_include_semantics(self.always_include_semantics)
            .into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(super::sliver_layer::OpacityRenderSliver::new(
            self.sliver.create_render_sliver(controller, axis, reverse),
            self.opacity,
            self.always_include_semantics,
        ))
    }
}

/// Sliver offstage wrapper.
#[derive(TypedBuilder)]
pub struct SliverOffstage {
    offstage: bool,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
}

impl SliverOffstage {
    #[must_use]
    pub fn new(offstage: bool, sliver: impl Sliver + 'static) -> Self {
        Self {
            offstage,
            sliver: Box::new(sliver),
        }
    }
}

impl Sliver for SliverOffstage {
    fn build(&self, controller: &ScrollController) -> Widget {
        crate::layout::Offstage::new(self.sliver.build(controller))
            .offstage(self.offstage)
            .into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let offstage = self.offstage;
        let inner = self.sliver.create_render_sliver(controller, axis, reverse);
        Box::new(WidgetWrapRenderSliver {
            inner: RefCell::new(inner),
            hidden: offstage,
            wrap: Rc::new(move |widget| {
                crate::layout::Offstage::new(widget)
                    .offstage(offstage)
                    .into()
            }),
        })
    }
}

/// Sliver ignore pointer wrapper.
#[derive(TypedBuilder)]
pub struct SliverIgnorePointer {
    ignoring: bool,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
}

impl SliverIgnorePointer {
    #[must_use]
    pub fn new(ignoring: bool, sliver: impl Sliver + 'static) -> Self {
        Self {
            ignoring,
            sliver: Box::new(sliver),
        }
    }
}

impl Sliver for SliverIgnorePointer {
    fn build(&self, controller: &ScrollController) -> Widget {
        crate::IgnorePointer::new(self.sliver.build(controller))
            .ignoring(self.ignoring)
            .into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let ignoring = self.ignoring;
        let inner = self.sliver.create_render_sliver(controller, axis, reverse);
        Box::new(WidgetWrapRenderSliver {
            inner: RefCell::new(inner),
            hidden: false,
            wrap: Rc::new(move |widget| {
                crate::IgnorePointer::new(widget).ignoring(ignoring).into()
            }),
        })
    }
}

/// A sliver that avoids the selected inherited safe-area insets.
#[derive(TypedBuilder)]
pub struct SliverSafeArea {
    #[builder(default = incular_config::MediaQueryEdges::ALL)]
    edges: incular_config::MediaQueryEdges,
    #[builder(default = EdgeInsets::ZERO)]
    minimum: EdgeInsets,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
}

impl SliverSafeArea {
    #[must_use]
    pub fn new(sliver: impl Sliver + 'static) -> Self {
        Self {
            edges: incular_config::MediaQueryEdges::ALL,
            minimum: EdgeInsets::ZERO,
            sliver: Box::new(sliver),
        }
    }

    #[must_use]
    pub fn left(mut self, enabled: bool) -> Self {
        self.edges.left = enabled;
        self
    }

    #[must_use]
    pub fn top(mut self, enabled: bool) -> Self {
        self.edges.top = enabled;
        self
    }

    #[must_use]
    pub fn right(mut self, enabled: bool) -> Self {
        self.edges.right = enabled;
        self
    }

    #[must_use]
    pub fn bottom(mut self, enabled: bool) -> Self {
        self.edges.bottom = enabled;
        self
    }

    #[must_use]
    pub fn minimum(mut self, minimum: EdgeInsets) -> Self {
        self.minimum = minimum;
        self
    }
}

impl Sliver for SliverSafeArea {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        crate::SafeArea::new(self.sliver.build_with_config(controller, axis, reverse))
            .sides(
                self.edges.left,
                self.edges.top,
                self.edges.right,
                self.edges.bottom,
            )
            .minimum(self.minimum)
            .into()
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let inner = self.sliver.create_render_sliver(controller, axis, reverse);
        Box::new(PaddingRenderSliver {
            inner: RefCell::new(inner),
            padding: crate::EdgeInsetsGeometry::Physical(EdgeInsets::ZERO),
            safe_padding: Some((self.minimum, self.edges)),
        })
    }
}

/// Sliver visibility wrapper (Flutter `SliverVisibility`).
///
/// A hidden sliver is replaced by `replacement_sliver` (an empty sliver by
/// default) unless it maintains its state. With `maintain_size` it keeps its
/// scroll extent, so following slivers do not move; without it, it takes no
/// extent. The flags follow Flutter's dependencies: size needs animation,
/// animation needs state, and interactivity and semantics need size.
///
/// # Panics
///
/// [`Sliver::build`] and [`Sliver::create_render_sliver`] panic when the
/// flags violate those dependencies. Use [`SliverVisibility::try_build`] and
/// [`SliverVisibility::try_create_render_sliver`] to handle that case.
#[derive(TypedBuilder)]
pub struct SliverVisibility {
    visible: bool,
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
    #[builder(default, setter(skip))]
    replacement_sliver: Option<Box<dyn Sliver>>,
    #[builder(default)]
    maintain_state: bool,
    #[builder(default)]
    maintain_animation: bool,
    #[builder(default)]
    maintain_size: bool,
    #[builder(default)]
    maintain_semantics: bool,
    #[builder(default)]
    maintain_interactivity: bool,
}

impl SliverVisibility {
    #[must_use]
    pub fn new(visible: bool, sliver: impl Sliver + 'static) -> Self {
        Self {
            visible,
            sliver: Box::new(sliver),
            replacement_sliver: None,
            maintain_state: false,
            maintain_animation: false,
            maintain_size: false,
            maintain_semantics: false,
            maintain_interactivity: false,
        }
    }

    /// Flutter `SliverVisibility.maintain`: keeps state, animation, size,
    /// semantics and interactivity while hidden.
    #[must_use]
    pub fn maintain(visible: bool, sliver: impl Sliver + 'static) -> Self {
        Self {
            maintain_state: true,
            maintain_animation: true,
            maintain_size: true,
            maintain_semantics: true,
            maintain_interactivity: true,
            ..Self::new(visible, sliver)
        }
    }

    /// Flutter `replacementSliver`: shown while hidden without maintained
    /// state. Ignored when state is maintained.
    #[must_use]
    pub fn replacement_sliver(mut self, sliver: impl Sliver + 'static) -> Self {
        self.replacement_sliver = Some(Box::new(sliver));
        self
    }

    #[must_use]
    pub fn maintain_state(mut self, maintain: bool) -> Self {
        self.maintain_state = maintain;
        self
    }

    #[must_use]
    pub fn maintain_animation(mut self, maintain: bool) -> Self {
        self.maintain_animation = maintain;
        self
    }

    #[must_use]
    pub fn maintain_size(mut self, maintain: bool) -> Self {
        self.maintain_size = maintain;
        self
    }

    #[must_use]
    pub fn maintain_semantics(mut self, maintain: bool) -> Self {
        self.maintain_semantics = maintain;
        self
    }

    #[must_use]
    pub fn maintain_interactivity(mut self, maintain: bool) -> Self {
        self.maintain_interactivity = maintain;
        self
    }

    /// Flutter's constructor assertions, checked when the sliver is built.
    fn check_flags(&self) -> Result<(), SliverVisibilityFlagError> {
        if self.maintain_interactivity && !self.maintain_size {
            return Err(SliverVisibilityFlagError::InteractivityNeedsSize);
        }
        if self.maintain_semantics && !self.maintain_size {
            return Err(SliverVisibilityFlagError::SemanticsNeedsSize);
        }
        if self.maintain_size && !self.maintain_animation {
            return Err(SliverVisibilityFlagError::SizeNeedsAnimation);
        }
        if self.maintain_animation && !self.maintain_state {
            return Err(SliverVisibilityFlagError::AnimationNeedsState);
        }
        Ok(())
    }

    /// Builds the sliver's widget, or reports an inconsistent flag
    /// combination.
    ///
    /// # Errors
    ///
    /// Returns a [`SliverVisibilityFlagError`] when a `maintain_*` flag is
    /// set without the flag it depends on.
    pub fn try_build(
        &self,
        controller: &ScrollController,
    ) -> Result<Widget, SliverVisibilityFlagError> {
        self.check_flags()?;
        if !self.maintain_state {
            return Ok(if self.visible {
                self.sliver.build(controller)
            } else if let Some(replacement) = &self.replacement_sliver {
                replacement.build(controller)
            } else {
                SizedBox::shrink().into()
            });
        }
        Ok(wrap_visibility(self.flags(), self.sliver.build(controller)))
    }

    /// Creates the retained render sliver, or reports an inconsistent flag
    /// combination.
    ///
    /// # Errors
    ///
    /// Returns a [`SliverVisibilityFlagError`] when a `maintain_*` flag is
    /// set without the flag it depends on.
    pub fn try_create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Result<Box<dyn RenderSliver>, SliverVisibilityFlagError> {
        self.check_flags()?;
        if !self.maintain_state {
            return Ok(if self.visible {
                self.sliver.create_render_sliver(controller, axis, reverse)
            } else if let Some(replacement) = &self.replacement_sliver {
                replacement.create_render_sliver(controller, axis, reverse)
            } else {
                // Nothing retained: do not lay out or run the concealed
                // sliver's lazy builder merely to replace its children.
                Box::new(EmptyRenderSliver)
            });
        }
        let inner = self.sliver.create_render_sliver(controller, axis, reverse);
        Ok(Box::new(WidgetWrapRenderSliver {
            inner: RefCell::new(inner),
            // A maintained size keeps the sliver's geometry; its children
            // are hidden by the box `Visibility` wrap.
            hidden: !self.visible && !self.maintain_size,
            wrap: {
                let flags = self.flags();
                Rc::new(move |widget| wrap_visibility(flags, widget))
            },
        }))
    }

    fn flags(&self) -> VisibilityFlags {
        VisibilityFlags {
            visible: self.visible,
            state: self.maintain_state,
            animation: self.maintain_animation,
            size: self.maintain_size,
            semantics: self.maintain_semantics,
            interactivity: self.maintain_interactivity,
        }
    }
}

/// The six `SliverVisibility` flags, copied into the wrap closure.
#[derive(Clone, Copy)]
struct VisibilityFlags {
    visible: bool,
    state: bool,
    animation: bool,
    size: bool,
    semantics: bool,
    interactivity: bool,
}

fn wrap_visibility(flags: VisibilityFlags, widget: Widget) -> Widget {
    crate::layout::Visibility::new(widget)
        .visible(flags.visible)
        .maintain_state(flags.state)
        .maintain_animation(flags.animation)
        .maintain_size(flags.size)
        .maintain_semantics(flags.semantics)
        .maintain_interactivity(flags.interactivity)
        .maintain_focusability(flags.state)
        .into_sliver_widget()
}

impl Sliver for SliverVisibility {
    /// # Panics
    ///
    /// Panics when the `maintain_*` flags are inconsistent. Use
    /// [`SliverVisibility::try_build`] to handle that case.
    fn build(&self, controller: &ScrollController) -> Widget {
        self.try_build(controller)
            .expect("SliverVisibility flags must be consistent; use try_build")
    }

    /// # Panics
    ///
    /// Panics when the `maintain_*` flags are inconsistent. Use
    /// [`SliverVisibility::try_create_render_sliver`] to handle that case.
    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        self.try_create_render_sliver(controller, axis, reverse)
            .expect("SliverVisibility flags must be consistent; use try_create_render_sliver")
    }
}

/// Which [`SliverVisibility`] `maintain_*` flag lacks the flag it depends on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SliverVisibilityFlagError {
    /// `maintain_interactivity` requires `maintain_size`.
    InteractivityNeedsSize,
    /// `maintain_semantics` requires `maintain_size`.
    SemanticsNeedsSize,
    /// `maintain_size` requires `maintain_animation`.
    SizeNeedsAnimation,
    /// `maintain_animation` requires `maintain_state`.
    AnimationNeedsState,
}

impl std::fmt::Display for SliverVisibilityFlagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InteractivityNeedsSize => {
                "SliverVisibility: maintain_interactivity requires maintain_size"
            }
            Self::SemanticsNeedsSize => {
                "SliverVisibility: maintain_semantics requires maintain_size"
            }
            Self::SizeNeedsAnimation => {
                "SliverVisibility: maintain_size requires maintain_animation"
            }
            Self::AnimationNeedsState => {
                "SliverVisibility: maintain_animation requires maintain_state"
            }
        })
    }
}

impl std::error::Error for SliverVisibilityFlagError {}

/// Pinned leading header sliver.
#[derive(TypedBuilder)]
pub struct PinnedHeaderSliver {
    #[builder(setter(into))]
    child: Widget,
}

impl PinnedHeaderSliver {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl Sliver for PinnedHeaderSliver {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        // Flutter's PinnedHeaderSliver derives its extent from the child's
        // laid-out size.  Do not impose the old arbitrary 48px extent here.
        Widget::persistent_header_with_config(
            controller.clone(),
            self.child.clone(),
            axis,
            reverse,
            true,
        )
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(BoxRenderSliver::pinned(self.child.clone()))
    }
}

/// Flutter's `FloatingHeaderSnapMode`: how a snapping header lays out while
/// it expands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FloatingHeaderSnapMode {
    /// The header expands over the scrollable content (Flutter default).
    #[default]
    Overlay,
    /// The header expands and the content keeps scrolling out of the way.
    Scroll,
}

/// Floating header sliver.
///
/// Snapping is on by default, as in Flutter: when a scroll activity ends with
/// the header partially revealed, scroll-activity ends animate it to the revealed
/// or hidden edge (whichever half is showing) over the snap duration and curve
/// (300 ms, ease-in-out by default). Snapping animates presentation only: the
/// logical scroll extent and controller offset never move. A new scroll
/// activity interrupts from the current presentation even before the offset
/// moves, and an end stranded inside that newer activity never starts a run.
/// Under reduced motion the endpoint resolves immediately without animating.
#[derive(TypedBuilder)]
pub struct SliverFloatingHeader {
    #[builder(setter(into))]
    child: Widget,
    #[builder(default = true)]
    snap: bool,
    #[builder(default)]
    snap_mode: FloatingHeaderSnapMode,
    #[builder(default = Duration::from_millis(300))]
    snap_duration: Duration,
    #[builder(default = Curve::EaseInOut)]
    snap_curve: Curve,
}

impl SliverFloatingHeader {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            snap: true,
            snap_mode: FloatingHeaderSnapMode::Overlay,
            snap_duration: Duration::from_millis(300),
            snap_curve: Curve::EaseInOut,
        }
    }

    /// Enables (default) or disables snap-to-edge when a scroll activity ends
    /// with the header partially revealed. A new activity interrupts the
    /// animation even before the offset moves, preserving the current
    /// presentation. Under reduced motion the endpoint resolves immediately
    /// without animating.
    #[must_use]
    pub fn snap(mut self, value: bool) -> Self {
        self.snap = value;
        self
    }

    /// Flutter's `snapMode`: `Overlay` (default) or `Scroll` layout while snapping.
    #[must_use]
    pub fn snap_mode(mut self, mode: FloatingHeaderSnapMode) -> Self {
        self.snap_mode = mode;
        self
    }

    /// Duration of snap animations (Flutter's `animationStyle.duration`).
    #[must_use]
    pub fn snap_duration(mut self, duration: Duration) -> Self {
        self.snap_duration = duration;
        self
    }

    /// Curve of snap animations (Flutter's `animationStyle.curve`).
    #[must_use]
    pub fn snap_curve(mut self, curve: Curve) -> Self {
        self.snap_curve = curve;
        self
    }
}

impl Sliver for SliverFloatingHeader {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        SliverPersistentHeader::new(
            widget_main_extent_hint(&self.child, axis)
                .unwrap_or(DEFAULT_LAZY_ITEM_EXTENT)
                .max(1.0),
            self.child.clone(),
        )
        .pinned(false)
        .build_with_config(controller, axis, reverse)
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(FloatingHeaderRenderSliver::new(
            controller,
            self.child.clone(),
            widget_main_extent_hint(&self.child, axis)
                .unwrap_or(DEFAULT_LAZY_ITEM_EXTENT)
                .max(1.),
            self.snap,
            self.snap_mode,
            self.snap_duration,
            self.snap_curve.clone(),
        ))
    }
}

/// Scroll behavior of a resizing header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliverHeaderScrollBehavior {
    /// Collapse, then scroll the minimum extent out of view.
    Scroll,
    /// Keep the collapsed header at the viewport edge.
    #[default]
    Pinned,
    /// Reveal the header immediately when scrolling reverses.
    Floating,
    /// Keep the minimum extent visible and expand on scroll reversal.
    FloatingPinned,
}

/// Response of a resizing header to leading overscroll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliverHeaderOverscrollBehavior {
    /// Preserve the header extent and translate with the viewport.
    #[default]
    Translate,
    /// Expand into leading overscroll without increasing logical scroll extent.
    Stretch,
}

/// Additional expansion bounds for a floating persistent header's
/// show-on-screen request. The header never shrinks to satisfy these bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PersistentHeaderShowOnScreenConfiguration {
    min_extent: f32,
    max_extent: f32,
}

impl Default for PersistentHeaderShowOnScreenConfiguration {
    fn default() -> Self {
        Self::new(f32::NEG_INFINITY, f32::INFINITY)
    }
}

impl PersistentHeaderShowOnScreenConfiguration {
    /// Values may be infinite; the header's own extents still limit expansion.
    ///
    /// # Panics
    ///
    /// Panics if either bound is NaN or the minimum exceeds the maximum.
    #[must_use]
    pub const fn new(min_show_on_screen_extent: f32, max_show_on_screen_extent: f32) -> Self {
        assert!(min_show_on_screen_extent <= max_show_on_screen_extent);
        Self {
            min_extent: min_show_on_screen_extent,
            max_extent: max_show_on_screen_extent,
        }
    }

    #[must_use]
    pub const fn min_show_on_screen_extent(self) -> f32 {
        self.min_extent
    }

    #[must_use]
    pub const fn max_show_on_screen_extent(self) -> f32 {
        self.max_extent
    }

    pub(super) fn limit(self, requested: f32) -> f32 {
        requested.clamp(self.min_extent, self.max_extent)
    }
}

/// Header that shrinks from its maximum to minimum extent while scrolling.
/// The default scroll behavior pins the collapsed header.
/// Negative and non-finite bounds become zero; the effective maximum is at
/// least the minimum. Constructor and typed-builder paths use the same policy.
/// When `snap` is set, scroll-activity ends animate a partially revealed
/// floating header to the revealed or hidden edge (a floating-pinned header
/// to expanded or collapsed) over the shared neutral snap timing. Snapping
/// engages only for Floating/FloatingPinned behavior and stays inert
/// otherwise; it animates presentation only and never moves the logical
/// scroll extent or controller offset.
#[derive(TypedBuilder)]
pub struct SliverResizingHeader {
    #[builder(setter(transform = |extent: f32| SliverResizingHeader::normalize_extent(extent)))]
    min_extent: f32,
    #[builder(setter(transform = |extent: f32| SliverResizingHeader::normalize_extent(extent)))]
    max_extent: f32,
    #[builder(setter(into))]
    child: Widget,
    #[builder(default)]
    scroll_behavior: SliverHeaderScrollBehavior,
    #[builder(default)]
    overscroll_behavior: SliverHeaderOverscrollBehavior,
    #[builder(default)]
    snap: bool,
    #[builder(default, setter(strip_option))]
    show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
    /// Flutter `SliverPersistentHeaderDelegate.build(context, shrinkOffset,
    /// overlapsContent)`: builds the child for each layout.
    #[builder(default, setter(skip))]
    delegate: Option<PersistentHeaderBuilder>,
}

/// Builds a persistent header's child from its shrink offset and whether it
/// overlaps the content after it.
pub type PersistentHeaderBuilder = Rc<dyn Fn(f32, bool) -> Widget>;

impl SliverResizingHeader {
    fn normalize_extent(extent: f32) -> f32 {
        finite_non_negative(extent)
    }

    #[must_use]
    pub fn new(min_extent: f32, max_extent: f32, child: impl Into<Widget>) -> Self {
        Self {
            min_extent: Self::normalize_extent(min_extent),
            max_extent: Self::normalize_extent(max_extent),
            child: child.into(),
            scroll_behavior: SliverHeaderScrollBehavior::Pinned,
            overscroll_behavior: SliverHeaderOverscrollBehavior::Translate,
            snap: false,
            show_on_screen_configuration: None,
            delegate: None,
        }
    }

    /// Flutter `SliverPersistentHeader(delegate: ...)`: a header between
    /// `min_extent` and `max_extent` whose child `build(shrink_offset,
    /// overlaps_content)` rebuilds on every layout, with the shrink offset
    /// Flutter passes (`min(scrollOffset, maxExtent)`, from the effective
    /// offset for floating headers). Scrolls away by default, as Flutter's
    /// `pinned: false, floating: false`; choose another behavior with
    /// [`scroll_behavior`](Self::scroll_behavior).
    #[must_use]
    pub fn with_delegate<W>(
        min_extent: f32,
        max_extent: f32,
        build: impl Fn(f32, bool) -> W + 'static,
    ) -> Self
    where
        W: Into<Widget>,
    {
        let delegate: PersistentHeaderBuilder =
            Rc::new(move |shrink, overlaps| build(shrink, overlaps).into());
        Self {
            child: delegate(0., false),
            scroll_behavior: SliverHeaderScrollBehavior::Scroll,
            delegate: Some(delegate),
            ..Self::new(min_extent, max_extent, SizedBox::shrink())
        }
    }

    #[must_use]
    pub fn scroll_behavior(mut self, behavior: SliverHeaderScrollBehavior) -> Self {
        self.scroll_behavior = behavior;
        self
    }

    /// Enables snap-to-edge when a scroll activity ends with the header
    /// partially revealed. Engages only for Floating/FloatingPinned behavior;
    /// inert for Scroll/Pinned. A new activity interrupts the animation even
    /// before the offset moves, preserving the current presentation. Under
    /// reduced motion the endpoint resolves immediately without animating.
    #[must_use]
    pub fn snap(mut self, value: bool) -> Self {
        self.snap = value;
        self
    }

    /// Lets a floating header expand to show a descendant's requested rect.
    /// Without this opt-in, requests only reveal through the viewport.
    #[must_use]
    pub fn show_on_screen_configuration(
        mut self,
        configuration: PersistentHeaderShowOnScreenConfiguration,
    ) -> Self {
        self.show_on_screen_configuration = Some(configuration);
        self
    }

    #[must_use]
    pub fn overscroll_behavior(mut self, behavior: SliverHeaderOverscrollBehavior) -> Self {
        self.overscroll_behavior = behavior;
        self
    }

    #[must_use]
    pub fn min_extent(&self) -> f32 {
        self.min_extent
    }

    #[must_use]
    pub fn max_extent(&self) -> f32 {
        self.max_extent.max(self.min_extent)
    }
}

impl Sliver for SliverResizingHeader {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        SliverPersistentHeader::new(self.max_extent(), self.child.clone())
            .pinned(matches!(
                self.scroll_behavior,
                SliverHeaderScrollBehavior::Pinned | SliverHeaderScrollBehavior::FloatingPinned
            ))
            .build_with_config(controller, axis, reverse)
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let floating = matches!(
            self.scroll_behavior,
            SliverHeaderScrollBehavior::Floating | SliverHeaderScrollBehavior::FloatingPinned
        );
        let reveal_configuration = self.show_on_screen_configuration.filter(|_| floating);
        let (snap_activity, snap_subscription) = snap_activity_subscription(
            controller,
            floating && (self.snap || reveal_configuration.is_some()),
        );
        Box::new(ResizingHeaderRenderSliver {
            child: self.child.clone(),
            delegate: self.delegate.clone(),
            min_extent: self.min_extent(),
            max_extent: self.max_extent(),
            scroll_behavior: self.scroll_behavior,
            overscroll_behavior: self.overscroll_behavior,
            scroll_state: HeaderScrollState::default(),
            snap: self.snap,
            persistent_controller: (floating
                && (self.delegate.is_some() || reveal_configuration.is_some()))
            .then(|| controller.clone()),
            reveal_configuration,
            reveal_frame: None,
            reduced_motion: Cell::new(false),
            snap_frame: Cell::new(HeaderSnapFrame::default()),
            snap_activity,
            _snap_subscription: snap_subscription,
        })
    }
}

/// Naturally measured header with typed scroll and overscroll policies.
///
/// The logical scroll extent is the child's measured main-axis extent under
/// an explicit validity lifecycle: an unverified estimate first, then a
/// measurement tied to the cross extent it was recorded under. Estimates
/// always measure unbounded first — even mid-overscroll — so stretched
/// samples can never become logical extent. Stretched layouts present
/// `natural + leading overlap` without growing the scroll range and return
/// to the current measurement when overscroll ends; a cross change demotes
/// back to an estimate for revalidation. Replacing a viewport descriptor
/// seeds the new header from the predecessor's validated value (never
/// validity itself), so equivalent replacements stay range-stable while
/// changed content re-establishes itself. The default scroll behavior pins
/// and the default overscroll translates. When `snap` is set, scroll-activity
/// ends animate a partially revealed floating header to the revealed or
/// hidden edge over the shared neutral snap timing. Snapping engages only
/// for Floating/FloatingPinned behavior and stays inert otherwise; a running
/// snap never survives descriptor replacement because the extent basis
/// revalidates, and it animates presentation only.
#[derive(TypedBuilder)]
pub struct SliverNaturalHeader {
    #[builder(setter(into))]
    child: Widget,
    #[builder(default)]
    scroll_behavior: SliverHeaderScrollBehavior,
    #[builder(default)]
    overscroll_behavior: SliverHeaderOverscrollBehavior,
    #[builder(default)]
    snap: bool,
}

impl SliverNaturalHeader {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            scroll_behavior: SliverHeaderScrollBehavior::Pinned,
            overscroll_behavior: SliverHeaderOverscrollBehavior::Translate,
            snap: false,
        }
    }

    #[must_use]
    pub fn scroll_behavior(mut self, behavior: SliverHeaderScrollBehavior) -> Self {
        self.scroll_behavior = behavior;
        self
    }

    /// Enables snap-to-edge when a scroll activity ends with the header
    /// partially revealed. Engages only for Floating/FloatingPinned behavior;
    /// inert for Scroll/Pinned. A new activity interrupts the animation even
    /// before the offset moves, preserving the current presentation. Under
    /// reduced motion the endpoint resolves immediately without animating.
    #[must_use]
    pub fn snap(mut self, value: bool) -> Self {
        self.snap = value;
        self
    }

    #[must_use]
    pub fn overscroll_behavior(mut self, behavior: SliverHeaderOverscrollBehavior) -> Self {
        self.overscroll_behavior = behavior;
        self
    }
}

impl Sliver for SliverNaturalHeader {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.build_with_config(controller, Axis::Vertical, false)
    }

    fn build_with_config(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Widget {
        match self.scroll_behavior {
            SliverHeaderScrollBehavior::Scroll => SliverToBoxAdapter::new(self.child.clone())
                .build_with_config(controller, axis, reverse),
            SliverHeaderScrollBehavior::Floating => SliverPersistentHeader::new(
                widget_main_extent_hint(&self.child, axis)
                    .unwrap_or(DEFAULT_LAZY_ITEM_EXTENT)
                    .max(1.),
                self.child.clone(),
            )
            .pinned(false)
            .build_with_config(controller, axis, reverse),
            SliverHeaderScrollBehavior::Pinned | SliverHeaderScrollBehavior::FloatingPinned => {
                Widget::persistent_header_with_config(
                    controller.clone(),
                    self.child.clone(),
                    axis,
                    reverse,
                    true,
                )
            }
        }
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        let floating = matches!(
            self.scroll_behavior,
            SliverHeaderScrollBehavior::Floating | SliverHeaderScrollBehavior::FloatingPinned
        );
        let (snap_activity, snap_subscription) =
            snap_activity_subscription(controller, self.snap && floating);
        Box::new(NaturalHeaderRenderSliver {
            child: self.child.clone(),
            scroll_behavior: self.scroll_behavior,
            overscroll_behavior: self.overscroll_behavior,
            extent: Cell::new(NaturalHeaderExtent::Estimate(
                widget_main_extent_hint(&self.child, axis)
                    .unwrap_or(DEFAULT_LAZY_ITEM_EXTENT)
                    .max(0.),
            )),
            presentation: Cell::new(HeaderPresentation::Settled),
            last_cross: Cell::new(0.),
            sample_unbounded: Cell::new(true),
            scroll_state: HeaderScrollState::default(),
            snap: self.snap,
            reduced_motion: Cell::new(false),
            snap_frame: Cell::new(HeaderSnapFrame::default()),
            snap_activity,
            _snap_subscription: snap_subscription,
        })
    }
}

/// Sliver overlap absorber for nested scroll view coordinators.
#[derive(TypedBuilder)]
pub struct SliverOverlapAbsorber {
    #[builder(setter(transform = |sliver: impl Sliver + 'static| {
        Box::new(sliver) as Box<dyn Sliver>
    }))]
    sliver: Box<dyn Sliver>,
    #[builder(default)]
    handle: SliverOverlapHandle,
}

impl SliverOverlapAbsorber {
    #[must_use]
    pub fn new(sliver: impl Sliver + 'static) -> Self {
        Self {
            sliver: Box::new(sliver),
            handle: SliverOverlapHandle::new(),
        }
    }

    /// Returns the handle consumed by an inner `SliverOverlapInjector`.
    #[must_use]
    pub fn handle(&self) -> SliverOverlapHandle {
        self.handle.clone()
    }
}

impl Sliver for SliverOverlapAbsorber {
    fn build(&self, controller: &ScrollController) -> Widget {
        self.sliver.build(controller)
    }

    fn create_render_sliver(
        &self,
        controller: &ScrollController,
        axis: Axis,
        reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(OverlapAbsorberRenderSliver {
            inner: RefCell::new(self.sliver.create_render_sliver(controller, axis, reverse)),
            handle: self.handle.clone(),
        })
    }
}

/// Sliver overlap injector for nested scroll view coordinators.
pub struct SliverOverlapInjector {
    handle: SliverOverlapHandle,
}

impl Default for SliverOverlapInjector {
    fn default() -> Self {
        Self::new()
    }
}

impl SliverOverlapInjector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handle: SliverOverlapHandle::new(),
        }
    }

    /// Creates an injector backed by an absorber's shared handle.
    #[must_use]
    pub fn with_handle(handle: SliverOverlapHandle) -> Self {
        Self { handle }
    }

    /// Naming-compatible constructor for code that mirrors Flutter's
    /// `SliverOverlapInjector` factory style.
    #[must_use]
    pub fn new_with_handle(handle: SliverOverlapHandle) -> Self {
        Self::with_handle(handle)
    }

    #[must_use]
    pub fn overlap_extent(&self) -> f32 {
        self.handle.extent()
    }
}

impl Sliver for SliverOverlapInjector {
    fn build(&self, _controller: &ScrollController) -> Widget {
        crate::layout::SizedBox::shrink().into()
    }

    fn create_render_sliver(
        &self,
        _controller: &ScrollController,
        _axis: Axis,
        _reverse: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(OverlapInjectorRenderSliver {
            handle: self.handle.clone(),
        })
    }
}
