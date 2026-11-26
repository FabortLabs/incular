use super::*;
use incular_core::finite_non_negative;

/// A first-class scrollable box that scrolls a single child.
#[derive(Clone, TypedBuilder)]
pub struct SingleChildScrollView {
    #[builder(setter(into))]
    child: Widget,
    #[builder(default = WidgetDefaults::DEFAULT.scroll_direction)]
    scroll_direction: Axis,
    #[builder(default = false)]
    reverse: bool,
    #[builder(default, setter(strip_option))]
    padding: Option<EdgeInsets>,
    #[builder(default, setter(strip_option))]
    controller: Option<ScrollController>,
    #[builder(default, setter(strip_option))]
    physics: Option<ScrollPhysics>,
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
}

impl SingleChildScrollView {
    /// Creates a scrollable single-child viewport.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            scroll_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            padding: None,
            controller: None,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
        }
    }

    /// Sets the scroll axis.
    #[must_use]
    pub fn scroll_direction(mut self, direction: Axis) -> Self {
        self.scroll_direction = direction;
        self
    }

    /// Returns the configured scroll axis.
    #[must_use]
    pub fn get_scroll_direction(&self) -> Axis {
        self.scroll_direction
    }

    /// Sets whether the scroll view scrolls in reverse.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Sets inner padding around the scrollable child.
    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Attaches an external [`ScrollController`].
    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Sets scroll physics.
    #[must_use]
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
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
}

/// The physics a view without its own inherits from the ambient `ScrollBehavior`.
#[derive(Clone, Copy)]
pub(super) enum InheritedPhysics {
    Scroll,
    Page,
}

/// Hands a view without physics of its own to the ambient `ScrollBehavior`.
pub(super) fn inherit_physics(widget: Widget, inherit: Option<InheritedPhysics>) -> Widget {
    match inherit {
        Some(InheritedPhysics::Page) => widget.inherit_scroll_physics(true),
        Some(InheritedPhysics::Scroll) => widget.inherit_scroll_physics(false),
        None => widget,
    }
}

impl From<SingleChildScrollView> for Widget {
    fn from(mut value: SingleChildScrollView) -> Self {
        let controller = value
            .controller
            .get_or_insert_with(ScrollController::default)
            .clone();
        let key = value.page_storage_key.take();
        let inherit = value.physics.is_none().then_some(InheritedPhysics::Scroll);
        keep_scroll_offset(
            inherit_physics(single_child_scroll_view_widget(value), inherit),
            &controller,
            key,
        )
    }
}
fn single_child_scroll_view_widget(value: SingleChildScrollView) -> Widget {
    let child = if let Some(padding) = value.padding {
        Padding::new(padding, value.child).into()
    } else {
        value.child
    };
    let controller = value.controller.unwrap_or_default();
    Widget::scroll_view_with_options(
        controller,
        child,
        value.scroll_direction,
        value.reverse,
        value.physics.unwrap_or_default(),
        value.clip_behavior,
        value.drag_start_behavior,
        value.hit_test_behavior,
    )
}

/// Delegate governing two-dimensional grid layout.
///
/// This is the Rust representation of Flutter's `SliverGridDelegate`. The
/// delegate owns grid policy; `GridView` and `SliverGrid` only provide the
/// child source and viewport. Keeping those responsibilities separate avoids
/// the old `cross_axis_count`/`row_extent` constructor workaround.
#[derive(Clone, Debug, PartialEq)]
pub enum SliverGridDelegate {
    FixedCrossAxisCount {
        cross_axis_count: usize,
        main_axis_spacing: f32,
        cross_axis_spacing: f32,
        child_aspect_ratio: f32,
        main_axis_extent: Option<f32>,
    },
    MaxCrossAxisExtent {
        max_cross_axis_extent: f32,
        main_axis_spacing: f32,
        cross_axis_spacing: f32,
        child_aspect_ratio: f32,
        main_axis_extent: Option<f32>,
    },
}

impl SliverGridDelegate {
    /// Creates a delegate with a fixed number of children on the cross axis.
    #[must_use]
    pub fn fixed_cross_axis_count(cross_axis_count: usize) -> Self {
        Self::FixedCrossAxisCount {
            cross_axis_count: cross_axis_count.max(1),
            main_axis_spacing: 0.,
            cross_axis_spacing: 0.,
            child_aspect_ratio: 1.,
            main_axis_extent: None,
        }
    }

    /// Creates a delegate that derives the cross-axis count from the
    /// available extent and a maximum tile extent.
    #[must_use]
    pub fn max_cross_axis_extent(max_cross_axis_extent: f32) -> Self {
        Self::MaxCrossAxisExtent {
            max_cross_axis_extent: if max_cross_axis_extent.is_finite() {
                max_cross_axis_extent.max(1.)
            } else {
                1.
            },
            main_axis_spacing: 0.,
            cross_axis_spacing: 0.,
            child_aspect_ratio: 1.,
            main_axis_extent: None,
        }
    }

    #[must_use]
    pub fn main_axis_spacing(mut self, spacing: f32) -> Self {
        let spacing = finite_non_negative(spacing);
        match &mut self {
            Self::FixedCrossAxisCount {
                main_axis_spacing, ..
            }
            | Self::MaxCrossAxisExtent {
                main_axis_spacing, ..
            } => *main_axis_spacing = spacing,
        }
        self
    }

    #[must_use]
    pub fn cross_axis_spacing(mut self, spacing: f32) -> Self {
        let spacing = finite_non_negative(spacing);
        match &mut self {
            Self::FixedCrossAxisCount {
                cross_axis_spacing, ..
            }
            | Self::MaxCrossAxisExtent {
                cross_axis_spacing, ..
            } => *cross_axis_spacing = spacing,
        }
        self
    }

    #[must_use]
    pub fn child_aspect_ratio(mut self, ratio: f32) -> Self {
        let ratio = if ratio.is_finite() {
            ratio.max(f32::EPSILON)
        } else {
            1.
        };
        match &mut self {
            Self::FixedCrossAxisCount {
                child_aspect_ratio, ..
            }
            | Self::MaxCrossAxisExtent {
                child_aspect_ratio, ..
            } => *child_aspect_ratio = ratio,
        }
        self
    }

    #[must_use]
    pub fn main_axis_extent(mut self, extent: f32) -> Self {
        let extent = extent.is_finite().then_some(extent.max(0.));
        match &mut self {
            Self::FixedCrossAxisCount {
                main_axis_extent, ..
            }
            | Self::MaxCrossAxisExtent {
                main_axis_extent, ..
            } => *main_axis_extent = extent,
        }
        self
    }

    pub(super) fn cross_axis_count(&self, available_extent: f32) -> usize {
        match *self {
            Self::FixedCrossAxisCount {
                cross_axis_count, ..
            } => cross_axis_count.max(1),
            Self::MaxCrossAxisExtent {
                max_cross_axis_extent,
                cross_axis_spacing,
                ..
            } => (available_extent.max(0.) / (max_cross_axis_extent + cross_axis_spacing))
                .ceil()
                .max(1.) as usize,
        }
    }

    pub(super) fn resolved_cross_axis_spacing(&self) -> f32 {
        match *self {
            Self::FixedCrossAxisCount {
                cross_axis_spacing, ..
            }
            | Self::MaxCrossAxisExtent {
                cross_axis_spacing, ..
            } => cross_axis_spacing,
        }
    }

    pub(super) fn resolved_main_axis_spacing(&self) -> f32 {
        match *self {
            Self::FixedCrossAxisCount {
                main_axis_spacing, ..
            }
            | Self::MaxCrossAxisExtent {
                main_axis_spacing, ..
            } => main_axis_spacing,
        }
    }

    pub(super) fn resolve_main_axis_extent(&self, cross_axis_extent: f32, columns: usize) -> f32 {
        let (main_axis_extent, child_aspect_ratio) = match *self {
            Self::FixedCrossAxisCount {
                main_axis_extent,
                child_aspect_ratio,
                ..
            }
            | Self::MaxCrossAxisExtent {
                main_axis_extent,
                child_aspect_ratio,
                ..
            } => (main_axis_extent, child_aspect_ratio),
        };
        if let Some(extent) = main_axis_extent {
            return extent;
        }
        let spacing = self.resolved_cross_axis_spacing() * columns.saturating_sub(1) as f32;
        (cross_axis_extent - spacing).max(0.)
            / columns.max(1) as f32
            / child_aspect_ratio.max(f32::EPSILON)
    }
}

impl Default for SliverGridDelegate {
    fn default() -> Self {
        Self::fixed_cross_axis_count(WidgetDefaults::DEFAULT.grid_cross_axis_count)
    }
}

/// Content strategies for [`ListView`].
#[derive(Clone)]
enum ListViewStrategy {
    Children(Vec<Widget>),
    Builder {
        item_count: usize,
        builder: Rc<dyn Fn(usize) -> Widget>,
        separated: bool,
    },
}

/// A first-class list descriptor supporting static children and lazy virtualized item builders.
#[derive(Clone, TypedBuilder)]
#[builder(builder_method(name = typed_builder))]
pub struct ListView {
    #[builder(default, setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
        children.into_iter().map(Into::into).collect::<Vec<Widget>>()
    }))]
    children: Vec<Widget>,
    #[builder(default, setter(skip))]
    strategy: Option<ListViewStrategy>,
    #[builder(default, setter(skip))]
    item_extent: Option<f32>,
    #[builder(default, setter(skip))]
    item_extent_builder: Option<Rc<dyn Fn(usize) -> f32>>,
    #[builder(default, setter(skip))]
    prototype_item: Option<Widget>,
    #[builder(default, setter(skip))]
    find_child_index: Option<FindChildIndex>,
    #[builder(default = WidgetDefaults::DEFAULT.scroll_direction)]
    scroll_direction: Axis,
    #[builder(default = false)]
    reverse: bool,
    #[builder(default = false)]
    shrink_wrap: bool,
    #[builder(default, setter(strip_option))]
    controller: Option<ScrollController>,
    #[builder(default, setter(strip_option))]
    padding: Option<EdgeInsets>,
    #[builder(default, setter(strip_option))]
    cache_extent: Option<f32>,
    #[builder(default, setter(strip_option))]
    physics: Option<ScrollPhysics>,
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
}

impl Default for ListView {
    fn default() -> Self {
        Self::typed_builder().build()
    }
}

impl ListView {
    /// Creates a ListView from an explicit collection of child widgets.
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            strategy: None,
            item_extent: None,
            item_extent_builder: None,
            prototype_item: None,
            find_child_index: None,
            scroll_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            shrink_wrap: false,
            controller: None,
            padding: None,
            cache_extent: None,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
        }
    }

    /// Creates a lazily virtualized ListView with a child builder closure.
    #[must_use]
    pub fn builder<W>(item_count: usize, builder: impl Fn(usize) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            children: Vec::new(),
            strategy: Some(ListViewStrategy::Builder {
                item_count,
                builder: Rc::new(move |i| builder(i).into()),
                separated: false,
            }),
            item_extent: None,
            item_extent_builder: None,
            prototype_item: None,
            find_child_index: None,
            scroll_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            shrink_wrap: false,
            controller: None,
            padding: None,
            cache_extent: None,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
        }
    }

    /// Creates a lazily virtualized ListView with alternating separator items.
    #[must_use]
    pub fn separated<W, S>(
        item_count: usize,
        item_builder: impl Fn(usize) -> W + 'static,
        separator_builder: impl Fn(usize) -> S + 'static,
    ) -> Self
    where
        W: Into<Widget> + 'static,
        S: Into<Widget> + 'static,
    {
        let total_count = if item_count == 0 {
            0
        } else {
            item_count * 2 - 1
        };
        let item_builder = Rc::new(move |i| item_builder(i).into());
        let separator_builder = Rc::new(move |i| separator_builder(i).into());
        Self {
            children: Vec::new(),
            strategy: Some(ListViewStrategy::Builder {
                item_count: total_count,
                separated: true,
                builder: Rc::new(move |index| {
                    if index % 2 == 0 {
                        item_builder(index / 2)
                    } else {
                        separator_builder(index / 2)
                    }
                }),
            }),
            item_extent: None,
            item_extent_builder: None,
            prototype_item: None,
            find_child_index: None,
            scroll_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            shrink_wrap: false,
            controller: None,
            padding: None,
            cache_extent: None,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
        }
    }

    /// Supplies the new sliver child index for a key when a lazy
    /// delegate is rebuilt. Separated lists count their separators here.
    /// The callback is not called on initial layout or ordinary scrolling.
    #[must_use]
    pub fn find_child_index_callback(
        mut self,
        callback: impl Fn(&WidgetKey) -> Option<usize> + 'static,
    ) -> Self {
        self.find_child_index = Some(Rc::new(callback));
        self
    }

    /// Supplies the new item index without counting separators.
    #[must_use]
    pub fn find_item_index_callback(
        mut self,
        callback: impl Fn(&WidgetKey) -> Option<usize> + 'static,
    ) -> Self {
        let stride = if matches!(self.strategy, Some(ListViewStrategy::Builder { separated: true, .. })) {
            2
        } else {
            1
        };
        self.find_child_index = Some(Rc::new(move |key| callback(key)?.checked_mul(stride)));
        self
    }

    /// Sets the scroll controller.
    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Sets inner padding.
    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Sets scroll direction.
    #[must_use]
    pub fn scroll_direction(mut self, direction: Axis) -> Self {
        self.scroll_direction = direction;
        self
    }

    /// Returns the configured scroll axis.
    #[must_use]
    pub fn get_scroll_direction(&self) -> Axis {
        self.scroll_direction
    }

    /// Sets reverse scrolling.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Sets whether the list sizes itself to its contents instead of
    /// requiring a bounded viewport extent.
    #[must_use]
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> Self {
        self.shrink_wrap = shrink_wrap;
        self
    }

    /// Sets cache extent in logical pixels.
    #[must_use]
    pub fn cache_extent(mut self, cache_extent: f32) -> Self {
        self.cache_extent = Some(finite_non_negative(cache_extent));
        self
    }

    /// Sets scroll physics.
    #[must_use]
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
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

    /// Sets fixed item extent.
    #[must_use]
    pub fn item_extent(mut self, extent: f32) -> Self {
        self.item_extent = extent.is_finite().then_some(extent.max(0.));
        self.item_extent_builder = None;
        self.prototype_item = None;
        self
    }

    /// Sets a prototype child item for measuring extent.
    #[must_use]
    pub fn prototype_item(mut self, prototype: impl Into<Widget>) -> Self {
        self.prototype_item = Some(prototype.into());
        self.item_extent = None;
        self.item_extent_builder = None;
        self
    }

    /// Sets the extent of each child using the same index as the item
    /// builder. The measured index remains internal to the retained sliver.
    #[must_use]
    pub fn item_extent_builder(mut self, builder: impl Fn(usize) -> f32 + 'static) -> Self {
        self.item_extent_builder = Some(Rc::new(builder));
        self.item_extent = None;
        self.prototype_item = None;
        self
    }
}

impl From<ListView> for Widget {
    fn from(mut value: ListView) -> Self {
        let controller = value
            .controller
            .get_or_insert_with(ScrollController::default)
            .clone();
        let key = value.page_storage_key.take();
        let inherit = value.physics.is_none().then_some(InheritedPhysics::Scroll);
        keep_scroll_offset(
            inherit_physics(list_view_widget(value), inherit),
            &controller,
            key,
        )
    }
}
fn list_view_widget(value: ListView) -> Widget {
    let controller = value.controller.unwrap_or_default();
    let item_extent = value.item_extent;
    let item_extent_builder = value.item_extent_builder;
    let prototype_item = value.prototype_item;
    let strategy = value
        .strategy
        .unwrap_or_else(|| ListViewStrategy::Children(value.children));
    let mut sliver: Box<dyn RenderSliver> = match strategy {
        ListViewStrategy::Children(children) => {
            // The explicit children already exist. Their unanimous tight
            // main-axis extent is exact, not a lazy measurement estimate.
            // Preserve an explicitly supplied extent, prototype, or builder.
            let item_extent = if item_extent_builder.is_none() && prototype_item.is_none() {
                item_extent
                    .or_else(|| uniform_explicit_item_extent(&children, value.scroll_direction))
            } else {
                item_extent
            };
            let children = Rc::new(children);
            let find_child_index: FindChildIndex = {
                let children = children.clone();
                let lookup = std::cell::OnceCell::new();
                Rc::new(move |key| {
                    let index = lookup.get_or_init(|| {
                        children
                            .iter()
                            .enumerate()
                            .filter_map(|(index, widget)| Some((widget.key()?.clone(), index)))
                            .collect::<HashMap<_, _>>()
                    });
                    index.get(key).copied()
                })
            };
            build_list_render_sliver(
                children.len(),
                Rc::new(move |index| children[index].clone()),
                value.scroll_direction,
                item_extent,
                item_extent_builder,
                prototype_item,
                Some(ListIndexLookup {
                    index: find_child_index,
                    include_active: false,
                }),
            )
        }
        ListViewStrategy::Builder {
            item_count,
            builder,
            ..
        } => build_list_render_sliver(
            item_count,
            builder,
            value.scroll_direction,
            item_extent,
            item_extent_builder,
            prototype_item,
            value.find_child_index.map(|index| ListIndexLookup {
                index,
                include_active: true,
            }),
        ),
    };

    if let Some(padding) = value.padding {
        sliver = Box::new(PaddingRenderSliver {
            inner: RefCell::new(sliver),
            padding: padding.into(),
            safe_padding: None,
        });
    }
    let semantic_child_count = sliver.child_count();
    single_sliver_viewport_with_options(
        SliverViewportOptions {
            controller,
            axis: value.scroll_direction,
            reverse: value.reverse,
            physics: value.physics.unwrap_or_default(),
            cache_extent: value.cache_extent.unwrap_or(DEFAULT_SLIVER_CACHE_EXTENT),
            clip_behavior: value.clip_behavior,
            drag_start_behavior: value.drag_start_behavior,
            hit_test_behavior: value.hit_test_behavior,
            shrink_wrap: value.shrink_wrap,
        },
        sliver,
    )
    .with_ambient_sliver_axis()
    .with_sliver_semantic_child_count(semantic_child_count)
}

fn uniform_explicit_item_extent(children: &[Widget], axis: Axis) -> Option<f32> {
    let extent = |item: &Widget| {
        let value = match item.kind() {
            WidgetKind::Box { size, .. } => axis.main_extent(*size),
            WidgetKind::Constrained { constraints, .. } => {
                let (min, max) = match axis {
                    Axis::Vertical => (constraints.min_height(), constraints.max_height()),
                    Axis::Horizontal => (constraints.min_width(), constraints.max_width()),
                };
                if min != max {
                    return None;
                }
                min
            }
            _ => return None,
        };
        (value.is_finite() && value > 0.).then_some(value)
    };
    let first = extent(children.first()?)?;
    children[1..]
        .iter()
        .all(|item| extent(item) == Some(first))
        .then_some(first)
}

fn build_list_render_sliver(
    item_count: usize,
    builder: Rc<dyn Fn(usize) -> Widget>,
    axis: Axis,
    item_extent: Option<f32>,
    item_extent_builder: Option<Rc<dyn Fn(usize) -> f32>>,
    prototype_item: Option<Widget>,
    find_child_index: Option<ListIndexLookup>,
) -> Box<dyn RenderSliver> {
    if let Some(extent) = item_extent {
        let mut sliver = FixedExtentRenderSliver::new(item_count, extent, builder);
        sliver.find_child_index = find_child_index;
        return Box::new(sliver);
    }
    if let Some(prototype) = prototype_item {
        let extent = widget_main_extent_hint(&prototype, axis)
            .unwrap_or(DEFAULT_LAZY_ITEM_EXTENT)
            .max(1.);
        let mut sliver = FixedExtentRenderSliver::new(item_count, extent, builder);
        sliver.find_child_index = find_child_index;
        return Box::new(sliver);
    }
    let index = if let Some(extent_builder) = item_extent_builder {
        MeasuredExtentIndex::with_estimates(item_count, DEFAULT_LAZY_ITEM_EXTENT, move |index| {
            extent_builder(index)
        })
    } else {
        MeasuredExtentIndex::new(item_count, DEFAULT_LAZY_ITEM_EXTENT)
    };
    let mut sliver = VariableExtentRenderSliver::new(index, builder);
    sliver.find_child_index = find_child_index;
    Box::new(sliver)
}

/// Content strategy for [`GridView`].
#[derive(Clone)]
enum GridViewStrategy {
    Count {
        delegate: SliverGridDelegate,
        children: Vec<Widget>,
    },
    Builder {
        item_count: usize,
        delegate: SliverGridDelegate,
        builder: Rc<dyn Fn(usize) -> Widget>,
    },
}

/// A first-class grid descriptor with bounded virtualization.
#[derive(Clone, TypedBuilder)]
#[builder(builder_method(name = typed_builder))]
pub struct GridView {
    #[builder(default, setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
        children.into_iter().map(Into::into).collect::<Vec<Widget>>()
    }))]
    children: Vec<Widget>,
    #[builder(default, setter(skip))]
    strategy: Option<GridViewStrategy>,
    #[builder(default)]
    grid_delegate: SliverGridDelegate,
    #[builder(default = WidgetDefaults::DEFAULT.scroll_direction)]
    scroll_direction: Axis,
    #[builder(default = false)]
    reverse: bool,
    #[builder(default = false)]
    shrink_wrap: bool,
    #[builder(default, setter(strip_option))]
    controller: Option<ScrollController>,
    #[builder(default, setter(strip_option))]
    padding: Option<EdgeInsets>,
    #[builder(default, setter(strip_option))]
    cache_extent: Option<f32>,
    #[builder(default, setter(strip_option))]
    physics: Option<ScrollPhysics>,
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
}

impl Default for GridView {
    fn default() -> Self {
        Self::typed_builder().build()
    }
}

impl GridView {
    /// Creates a grid from a Flutter-style sliver grid delegate and static
    /// children.
    #[must_use]
    pub fn new(
        grid_delegate: SliverGridDelegate,
        children: impl IntoIterator<Item = impl Into<Widget>>,
    ) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            strategy: None,
            grid_delegate,
            scroll_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            shrink_wrap: false,
            controller: None,
            padding: None,
            cache_extent: None,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
        }
    }

    /// Creates a grid whose column count is derived from the available
    /// cross-axis extent.
    #[must_use]
    pub fn extent(
        max_cross_axis_extent: f32,
        children: impl IntoIterator<Item = impl Into<Widget>>,
    ) -> Self {
        Self::new(
            SliverGridDelegate::max_cross_axis_extent(max_cross_axis_extent),
            children,
        )
    }

    /// Creates a fixed cross-axis count grid from static children.
    #[must_use]
    pub fn count(
        cross_axis_count: usize,
        children: impl IntoIterator<Item = impl Into<Widget>>,
    ) -> Self {
        Self::new(
            SliverGridDelegate::fixed_cross_axis_count(cross_axis_count),
            children,
        )
    }

    /// Creates a lazily virtualized grid with a row-major builder and a
    /// Flutter-style sliver grid delegate.
    #[must_use]
    pub fn builder<W>(
        item_count: usize,
        grid_delegate: SliverGridDelegate,
        builder: impl Fn(usize) -> W + 'static,
    ) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            children: Vec::new(),
            strategy: Some(GridViewStrategy::Builder {
                item_count,
                delegate: grid_delegate,
                builder: Rc::new(move |i| builder(i).into()),
            }),
            grid_delegate: SliverGridDelegate::default(),
            scroll_direction: WidgetDefaults::DEFAULT.scroll_direction,
            reverse: false,
            shrink_wrap: false,
            controller: None,
            padding: None,
            cache_extent: None,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
        }
    }

    /// Attaches a scroll controller.
    #[must_use]
    pub fn controller(mut self, controller: ScrollController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Sets inner padding.
    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
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

    /// Sets the sliver grid delegate for a static-child grid.
    #[must_use]
    pub fn grid_delegate(mut self, delegate: SliverGridDelegate) -> Self {
        self.grid_delegate = delegate;
        self
    }

    /// Sets whether the grid sizes itself to its contents.
    #[must_use]
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> Self {
        self.shrink_wrap = shrink_wrap;
        self
    }

    /// Sets the lazy child cache in logical pixels.
    #[must_use]
    pub fn cache_extent(mut self, cache_extent: f32) -> Self {
        self.cache_extent = Some(finite_non_negative(cache_extent));
        self
    }

    /// Sets scroll physics for the retained grid viewport.
    #[must_use]
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
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

    #[must_use]
    pub fn get_scroll_direction(&self) -> Axis {
        self.scroll_direction
    }

    #[must_use]
    pub fn is_reverse(&self) -> bool {
        self.reverse
    }
}

impl From<GridView> for Widget {
    fn from(mut value: GridView) -> Self {
        let controller = value
            .controller
            .get_or_insert_with(ScrollController::default)
            .clone();
        let key = value.page_storage_key.take();
        let inherit = value.physics.is_none().then_some(InheritedPhysics::Scroll);
        keep_scroll_offset(
            inherit_physics(grid_view_widget(value), inherit),
            &controller,
            key,
        )
    }
}
fn grid_view_widget(value: GridView) -> Widget {
    let controller = value.controller.unwrap_or_default();
    let mut sliver: Box<dyn RenderSliver> =
        match value.strategy.unwrap_or_else(|| GridViewStrategy::Count {
            delegate: value.grid_delegate,
            children: value.children,
        }) {
            GridViewStrategy::Count { delegate, children } => {
                let children = Rc::new(children);
                Box::new(GridRenderSliver::new(
                    children.len(),
                    delegate,
                    Rc::new(move |index| children[index].clone()),
                    value.scroll_direction,
                ))
            }
            GridViewStrategy::Builder {
                item_count,
                delegate,
                builder,
            } => Box::new(GridRenderSliver::new(
                item_count,
                delegate,
                builder,
                value.scroll_direction,
            )),
        };

    if let Some(padding) = value.padding {
        sliver = Box::new(PaddingRenderSliver {
            inner: RefCell::new(sliver),
            padding: padding.into(),
            safe_padding: None,
        });
    }
    let semantic_child_count = sliver.child_count();
    single_sliver_viewport_with_options(
        SliverViewportOptions {
            controller,
            axis: value.scroll_direction,
            reverse: value.reverse,
            physics: value.physics.unwrap_or_default(),
            cache_extent: value.cache_extent.unwrap_or(DEFAULT_SLIVER_CACHE_EXTENT),
            clip_behavior: value.clip_behavior,
            drag_start_behavior: value.drag_start_behavior,
            hit_test_behavior: value.hit_test_behavior,
            shrink_wrap: value.shrink_wrap,
        },
        sliver,
    )
    .with_ambient_sliver_axis()
    .with_sliver_semantic_child_count(semantic_child_count)
}

/// Controller for paged views (Flutter `PageController`).
///
/// It derefs to the [`ScrollController`] that drives the view, so offset
/// operations keep working, and adds the page-based API: `initialPage`,
/// `viewportFraction`, the `page` getter, `jumpToPage`, `animateToPage`,
/// `nextPage` and `previousPage`.
#[derive(Clone)]
pub struct PageController {
    scroll: ScrollController,
    initial_page: usize,
    /// `None` leaves the view's own `viewport_fraction` in effect.
    viewport_fraction: Option<f32>,
    /// Flutter `_lastReportedPage`: the page last given to `on_page_changed`,
    /// starting at the initial page so mounting never reports a change.
    last_reported_page: Rc<std::cell::Cell<usize>>,
}

impl std::ops::Deref for PageController {
    type Target = ScrollController;

    fn deref(&self) -> &ScrollController {
        &self.scroll
    }
}

impl From<ScrollController> for PageController {
    fn from(scroll: ScrollController) -> Self {
        Self::from_scroll(scroll)
    }
}

impl Default for PageController {
    fn default() -> Self {
        Self::from_scroll(ScrollController::default())
    }
}

impl std::fmt::Debug for PageController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageController")
            .field("scroll", &self.scroll)
            .field("initial_page", &self.initial_page)
            .field("viewport_fraction", &self.viewport_fraction)
            .finish()
    }
}

impl PartialEq for PageController {
    fn eq(&self, other: &Self) -> bool {
        self.scroll == other.scroll
    }
}

impl PageController {
    /// Creates a page controller starting at page zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a page controller that restores its page, if present.
    #[must_use]
    pub fn restored(
        scope: incular_core::RestorationScope,
        key: incular_core::RestorationKey,
    ) -> Self {
        Self::from_scroll(ScrollController::restored(scope, key))
    }

    fn from_scroll(scroll: ScrollController) -> Self {
        Self {
            scroll,
            initial_page: 0,
            viewport_fraction: None,
            last_reported_page: Rc::new(std::cell::Cell::new(0)),
        }
    }

    /// The scroll controller that drives the view.
    #[must_use]
    pub fn scroll_controller(&self) -> ScrollController {
        self.scroll.clone()
    }

    /// Invokes the callback when this controller first attaches to a
    /// mounted PageView, before the fresh position has content dimensions.
    #[must_use]
    pub fn with_on_attach(mut self, callback: impl Fn(&ScrollController) + 'static) -> Self {
        self.scroll = self.scroll.with_on_attach(callback);
        self
    }

    /// Invokes the callback before a mounted PageView releases this
    /// controller. It can still inspect the position during the callback.
    #[must_use]
    pub fn with_on_detach(mut self, callback: impl Fn(&ScrollController) + 'static) -> Self {
        self.scroll = self.scroll.with_on_detach(callback);
        self
    }

    /// Flutter `PageController.initialPage`: the page the view starts on.
    #[must_use]
    pub fn with_initial_page(mut self, page: usize) -> Self {
        self.initial_page = page;
        self.last_reported_page.set(page);
        self.defer_initial_page();
        self
    }

    /// Flutter `PageController.viewportFraction`: the fraction of the
    /// viewport each page occupies. Overrides the view's own fraction.
    #[must_use]
    pub fn with_viewport_fraction(mut self, fraction: f32) -> Self {
        if fraction.is_finite() && fraction > 0. {
            self.viewport_fraction = Some(fraction);
            self.defer_initial_page();
        }
        self
    }

    /// The configured initial page.
    #[must_use]
    pub fn initial_page(&self) -> usize {
        self.initial_page
    }

    /// The viewport fraction in effect; `1.0` unless one was configured.
    #[must_use]
    pub fn viewport_fraction(&self) -> f32 {
        self.viewport_fraction.unwrap_or(1.)
    }

    /// The fraction configured on this controller, if any.
    #[must_use]
    pub(crate) fn configured_viewport_fraction(&self) -> Option<f32> {
        self.viewport_fraction
    }

    fn defer_initial_page(&self) {
        // Page zero still matters when the fraction is not one: Flutter
        // centres it by the same pad.
        if self.initial_page != 0 || self.viewport_fraction.is_some() {
            self.scroll
                .configure_initial_page(self.initial_page as f32, self.viewport_fraction());
        }
    }

    /// Flutter `PageController.page`: the fractional page of the live
    /// position. A detached or not-yet-laid-out view returns `None`.
    #[must_use]
    pub fn page(&self) -> Option<f32> {
        self.scroll.page_for_controller(self.viewport_fraction())
    }

    /// Explicitly queues the initial selection for a future PageView.
    /// This Incular extension can override saved PageStorage state. The
    /// Flutter-style movement methods instead require a live attachment.
    pub fn queue_initial_page(&self, page: usize) -> bool {
        self.scroll
            .defer_page(page as f32, self.viewport_fraction())
    }

    /// Flutter `jumpToPage`: moves to `page` without animation. Returns
    /// false without changing state if no PageView is attached. An
    /// attached position with no viewport dimensions defers the request.
    pub fn jump_to_page(&self, page: usize) -> bool {
        if !self.scroll.has_page_client() {
            return false;
        }
        if let Some(viewport) = self.scroll.live_page_viewport_extent() {
            self.scroll.jump_to(incular_scroll::pixels_for_page(
                page as f32,
                viewport,
                self.viewport_fraction(),
            ))
        } else {
            self.queue_initial_page(page)
        }
    }

    /// Flutter `animateToPage`: animates to `page` on a live PageView.
    /// Returns false without changing state when detached. An attached
    /// position can defer the target until it has viewport dimensions.
    pub fn animate_to_page(
        &self,
        page: usize,
        duration: std::time::Duration,
        curve: incular_animation::Curve,
    ) -> bool {
        if !self.scroll.has_page_client() {
            return false;
        }
        if let Some(viewport) = self.scroll.live_page_viewport_extent() {
            self.scroll.animate_to(
                incular_scroll::pixels_for_page(page as f32, viewport, self.viewport_fraction()),
                duration,
                curve,
            )
        } else {
            self.queue_initial_page(page)
        }
    }

    /// Flutter `nextPage`: animates to the page after the rounded current page.
    pub fn next_page(
        &self,
        duration: std::time::Duration,
        curve: incular_animation::Curve,
    ) -> bool {
        let current = self.page().unwrap_or(self.initial_page as f32).round() as usize;
        self.animate_to_page(current + 1, duration, curve)
    }

    /// Flutter `previousPage`: animates to the page before the rounded current
    /// page, stopping at page zero.
    pub fn previous_page(
        &self,
        duration: std::time::Duration,
        curve: incular_animation::Curve,
    ) -> bool {
        let current = self.page().unwrap_or(self.initial_page as f32).round() as usize;
        self.animate_to_page(current.saturating_sub(1), duration, curve)
    }

    /// Reports `page` through `on_page_changed` if it differs from the page
    /// last reported. Returns the page when it changed. Framework-internal.
    #[doc(hidden)]
    pub fn take_page_change(
        &self,
        pixels: f32,
        max: f32,
        viewport: f32,
        fraction: f32,
    ) -> Option<usize> {
        let page = incular_scroll::page_for_pixels(pixels.clamp(0., max), viewport, fraction)
            .round()
            .max(0.) as usize;
        if page == self.last_reported_page.get() {
            return None;
        }
        self.last_reported_page.set(page);
        Some(page)
    }
}

/// Content strategy for [`PageView`].
#[derive(Clone)]
enum PageViewStrategy {
    Children(Vec<Widget>),
    Builder {
        page_count: usize,
        builder: Rc<dyn Fn(usize) -> Widget>,
        find_child_index: Option<FindChildIndex>,
    },
}

/// A first-class paged scrollable descriptor.
#[derive(Clone, TypedBuilder)]
#[builder(builder_method(name = typed_builder))]
pub struct PageView {
    #[builder(default, setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
        children.into_iter().map(Into::into).collect::<Vec<Widget>>()
    }))]
    children: Vec<Widget>,
    #[builder(default, setter(skip))]
    strategy: Option<PageViewStrategy>,
    #[builder(default, setter(strip_option))]
    controller: Option<PageController>,
    #[builder(default = WidgetDefaults::DEFAULT.page_scroll_direction)]
    scroll_direction: Axis,
    #[builder(default = false)]
    reverse: bool,
    #[builder(default = true)]
    page_snapping: bool,
    #[builder(default)]
    allow_implicit_scrolling: bool,
    #[builder(default, setter(strip_option))]
    scroll_cache_extent: Option<ScrollCacheExtent>,
    #[builder(default = 1.0, setter(transform = |value: f32| value.max(0.01)))]
    viewport_fraction: f32,
    /// Flutter `padEnds`: with a `viewport_fraction` below one, the first
    /// and last pages are centered. Defaults to true.
    #[builder(default = true)]
    pad_ends: bool,
    #[builder(default, setter(strip_option))]
    physics: Option<ScrollPhysics>,
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
    /// Flutter `onPageChanged`, called with each new rounded page.
    #[builder(default, setter(skip))]
    on_page_changed: Option<Rc<dyn Fn(usize)>>,
}

impl Default for PageView {
    fn default() -> Self {
        Self::typed_builder().build()
    }
}

impl PageView {
    /// Creates a PageView from static children pages.
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            strategy: None,
            controller: None,
            scroll_direction: WidgetDefaults::DEFAULT.page_scroll_direction,
            reverse: false,
            page_snapping: true,
            allow_implicit_scrolling: false,
            scroll_cache_extent: None,
            viewport_fraction: 1.0,
            pad_ends: true,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
            on_page_changed: None,
        }
    }

    /// Creates a lazily virtualized PageView with a page builder closure.
    #[must_use]
    pub fn builder<W>(page_count: usize, builder: impl Fn(usize) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            children: Vec::new(),
            strategy: Some(PageViewStrategy::Builder {
                page_count,
                builder: Rc::new(move |i| builder(i).into()),
                find_child_index: None,
            }),
            controller: None,
            scroll_direction: WidgetDefaults::DEFAULT.page_scroll_direction,
            reverse: false,
            page_snapping: true,
            allow_implicit_scrolling: false,
            scroll_cache_extent: None,
            viewport_fraction: 1.0,
            pad_ends: true,
            physics: None,
            clip_behavior: WidgetDefaults::DEFAULT.scroll_clip_behavior,
            drag_start_behavior: DragStartBehavior::Start,
            hit_test_behavior: crate::gestures::HitTestBehavior::Opaque,
            page_storage_key: None,
            on_page_changed: None,
        }
    }

    /// Resolves a previously materialized [`Widget::key`] after reordering
    /// the pages returned by [`PageView::builder`]. Return the key's new
    /// index, or `None` if it no longer exists. This lets a kept-alive
    /// page carry its state to its new index without rebuilding every item.
    /// Has no effect on a PageView constructed from static children.
    #[must_use]
    pub fn find_child_index_callback(
        mut self,
        callback: impl Fn(&WidgetKey) -> Option<usize> + 'static,
    ) -> Self {
        if let Some(PageViewStrategy::Builder {
            find_child_index, ..
        }) = &mut self.strategy
        {
            *find_child_index = Some(Rc::new(callback));
        }
        self
    }

    /// Flutter `PageView.onPageChanged`: called with the new rounded page each
    /// time it changes during a scroll update. Mounting does not report it.
    #[must_use]
    pub fn on_page_changed(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_page_changed = Some(Rc::new(callback));
        self
    }

    /// Attaches a PageController.
    #[must_use]
    pub fn controller(mut self, controller: impl Into<PageController>) -> Self {
        self.controller = Some(controller.into());
        self
    }

    /// Sets scroll direction.
    #[must_use]
    pub fn scroll_direction(mut self, direction: Axis) -> Self {
        self.scroll_direction = direction;
        self
    }

    /// Sets whether the page view scrolls in reverse.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Sets whether page snapping is enabled.
    #[must_use]
    pub fn page_snapping(mut self, page_snapping: bool) -> Self {
        self.page_snapping = page_snapping;
        self
    }

    /// Allows implicit reveal and access to cached offscreen pages.
    #[must_use]
    pub fn allow_implicit_scrolling(mut self, allow: bool) -> Self {
        self.allow_implicit_scrolling = allow;
        self
    }

    /// Sets the amount of offscreen content cached by this view. A nonzero
    /// extent requires implicit scrolling and vice versa.
    #[must_use]
    pub fn scroll_cache_extent(mut self, extent: ScrollCacheExtent) -> Self {
        self.scroll_cache_extent = Some(extent);
        self
    }

    #[must_use]
    pub fn effective_scroll_cache_extent(&self) -> ScrollCacheExtent {
        self.scroll_cache_extent.unwrap_or_else(|| {
            ScrollCacheExtent::viewport(if self.allow_implicit_scrolling {
                1.
            } else {
                0.
            })
        })
    }

    /// Sets the fraction of the viewport occupied by each page.
    #[must_use]
    pub fn viewport_fraction(mut self, value: f32) -> Self {
        self.viewport_fraction = value.max(0.01);
        self
    }

    /// Installs the scroll physics used by the retained page viewport.
    #[must_use]
    pub fn physics(mut self, value: ScrollPhysics) -> Self {
        self.physics = Some(value);
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

    #[must_use]
    pub fn is_reverse(&self) -> bool {
        self.reverse
    }

    #[must_use]
    pub fn is_page_snapping(&self) -> bool {
        self.page_snapping
    }

    #[must_use]
    pub fn viewport_fraction_value(&self) -> f32 {
        self.viewport_fraction
    }

    /// Sets Flutter's `padEnds` (default true).
    #[must_use]
    pub fn pad_ends(mut self, pad_ends: bool) -> Self {
        self.pad_ends = pad_ends;
        self
    }

    /// Returns the configured page axis.
    #[must_use]
    pub fn get_scroll_direction(&self) -> Axis {
        self.scroll_direction
    }
}

/// The default controller belongs to the mounted view. Rebuilding a
/// descriptor (including toggling page-snapping physics) is not a remount.
struct MountedPageView {
    implicit_controller: RefCell<Option<PageController>>,
    last_reported_page: Cell<usize>,
    storage: RefCell<Option<PageStorageBinding>>,
}

struct PageStorageBinding {
    bucket: crate::PageStorageBucket,
    key: crate::PageStorageKey,
    controller: ScrollController,
    _listener: ScrollNotificationSubscription,
}

impl MountedPageView {
    fn sync_storage(
        &self,
        context: &crate::BuildContext<'_>,
        key: Option<crate::PageStorageKey>,
        controller: &PageController,
        fraction: f32,
    ) {
        let scroll = controller.scroll_controller();
        let new_position = scroll.metric_owner().is_none();
        if new_position {
            scroll.configure_initial_page(controller.initial_page() as f32, fraction);
        }
        let target = key
            .filter(|_| scroll.keep_scroll_offset())
            .and_then(|key| Some((crate::PageStorage::maybe_of(context)?, key)));
        let previous = self.storage.borrow_mut().take();
        let Some((bucket, key)) = target else {
            return;
        };
        if previous.as_ref().is_some_and(|binding| {
            binding.bucket == bucket && binding.key == key && binding.controller == scroll
        }) {
            *self.storage.borrow_mut() = previous;
            return;
        }
        // Restoration initializes a fresh position, not a retained position
        // during a descriptor rebuild or an ancestor update.
        if new_position && let Some(page) = bucket.read_state(&key).and_then(|value| value.as_f64())
        {
            scroll.restore_page_from_storage(page as f32, fraction);
        }
        let listener = scroll.add_listener({
            let bucket = bucket.clone();
            let key = key.clone();
            let scroll = scroll.clone();
            move |notification| {
                let settled = match notification.kind {
                    ScrollNotificationType::End => true,
                    ScrollNotificationType::Update => scroll.current_activity_id().is_none(),
                    _ => false,
                };
                if settled && let Some(page) = scroll.page_for_handoff() {
                    bucket.write_state(&key, f64::from(page));
                }
                false
            }
        });
        *self.storage.borrow_mut() = Some(PageStorageBinding {
            bucket,
            key,
            controller: scroll,
            _listener: listener,
        });
    }
}

impl crate::tree::RetainedState for MountedPageView {
    fn rebind_revision(&self, _: Rc<Cell<u64>>) {}
}

impl From<PageView> for Widget {
    fn from(value: PageView) -> Self {
        debug_assert!(
            value
                .scroll_cache_extent
                .is_none_or(|extent| { (extent.value() > 0.) == value.allow_implicit_scrolling }),
            "scroll cache extent and implicit scrolling must be consistent"
        );
        let initial_page = value
            .controller
            .as_ref()
            .map_or(0, PageController::initial_page);
        let key = value.page_storage_key.clone();
        let widget = Widget::mount_scoped_builder(
            move |_| MountedPageView {
                implicit_controller: RefCell::new(None),
                last_reported_page: Cell::new(initial_page),
                storage: RefCell::new(None),
            },
            move |state, context| build_mounted_page_view(value.clone(), state, context),
        );
        if let Some(key) = key {
            widget.with_key(key)
        } else {
            widget
        }
    }
}

fn build_mounted_page_view(
    mut value: PageView,
    state: &Rc<MountedPageView>,
    context: &crate::BuildContext<'_>,
) -> Widget {
    let controller = {
        let mut implicit = state.implicit_controller.borrow_mut();
        if let Some(explicit) = value.controller.as_ref() {
            implicit.take();
            explicit.clone()
        } else {
            implicit.get_or_insert_with(PageController::default).clone()
        }
    };
    if let Some(fraction) = controller.configured_viewport_fraction() {
        value.viewport_fraction = fraction.max(0.01);
    }
    let fraction = value.viewport_fraction;
    let on_page_changed = value.on_page_changed.take();
    let key = value.page_storage_key.take();
    let inherit = value.physics.is_none().then_some(if value.page_snapping {
        InheritedPhysics::Page
    } else {
        InheritedPhysics::Scroll
    });
    state.sync_storage(context, key, &controller, fraction);
    value.controller = Some(controller);
    let view = inherit_physics(page_view_widget(value), inherit);
    let listener = on_page_changed.map(|callback| {
        let state = state.clone();
        Rc::new(move |notification: ScrollNotification| {
            if notification.depth == 0
                && notification.kind == incular_scroll::ScrollNotificationType::Update
                && notification.metrics.viewport_dimension > 0.
            {
                let metrics = notification.metrics;
                let page = incular_scroll::page_for_pixels(
                    metrics.pixels.clamp(0., metrics.max_scroll_extent),
                    metrics.viewport_dimension,
                    fraction,
                )
                .round()
                .max(0.) as usize;
                if state.last_reported_page.replace(page) != page {
                    callback(page);
                }
            }
            false
        }) as Rc<dyn Fn(ScrollNotification) -> bool>
    });
    Widget::notification_listener(listener, view)
}

fn page_view_widget(value: PageView) -> Widget {
    let cache_extent = value.effective_scroll_cache_extent();
    let page = value.controller.unwrap_or_default();
    let controller = page.scroll_controller();
    let fraction = value.viewport_fraction.max(0.01);
    // Flutter composes page snapping on the resolved physics (user
    // physics or the default), it never replaces them.
    let physics_for_extent = || {
        let physics = value
            .physics
            .unwrap_or_default()
            .with_implicit_scrolling(value.allow_implicit_scrolling);
        if value.page_snapping {
            physics.with_page_viewport_fraction(fraction)
        } else {
            physics.with_page_geometry(fraction)
        }
    };
    match value
        .strategy
        .unwrap_or_else(|| PageViewStrategy::Children(value.children))
    {
        PageViewStrategy::Children(children) => {
            let sliver = SliverFillViewport::new(children)
                .viewport_fraction(fraction)
                .pad_ends(value.pad_ends)
                .allow_implicit_scrolling(value.allow_implicit_scrolling)
                .fallback_extent(WidgetDefaults::DEFAULT.sliver_fill_viewport_extent);
            let render =
                sliver.create_render_sliver(&controller, value.scroll_direction, value.reverse);
            single_sliver_viewport_with_options(
                SliverViewportOptions {
                    controller,
                    axis: value.scroll_direction,
                    reverse: value.reverse,
                    physics: physics_for_extent(),
                    cache_extent: 0.,
                    clip_behavior: value.clip_behavior,
                    drag_start_behavior: value.drag_start_behavior,
                    hit_test_behavior: value.hit_test_behavior,
                    shrink_wrap: false,
                },
                render,
            )
        }
        PageViewStrategy::Builder {
            page_count,
            builder,
            find_child_index,
        } => {
            let sliver = SliverFillViewport::builder(page_count, move |index| builder(index))
                .with_child_index_lookup(find_child_index)
                .viewport_fraction(fraction)
                .pad_ends(value.pad_ends)
                .allow_implicit_scrolling(value.allow_implicit_scrolling)
                .fallback_extent(WidgetDefaults::DEFAULT.sliver_fill_viewport_extent);
            let render =
                sliver.create_render_sliver(&controller, value.scroll_direction, value.reverse);
            single_sliver_viewport_with_options(
                SliverViewportOptions {
                    controller,
                    axis: value.scroll_direction,
                    reverse: value.reverse,
                    physics: physics_for_extent(),
                    cache_extent: 0.,
                    clip_behavior: value.clip_behavior,
                    drag_start_behavior: value.drag_start_behavior,
                    hit_test_behavior: value.hit_test_behavior,
                    shrink_wrap: false,
                },
                render,
            )
        }
    }
    .with_sliver_cache_extent(cache_extent)
    .with_exact_sliver_cache_window()
    .with_ambient_sliver_axis()
}
