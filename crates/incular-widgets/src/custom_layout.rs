//! Delegate-driven layouts: Flutter's `CustomSingleChildLayout`,
//! `CustomMultiChildLayout` with `LayoutId` (`widgets/basic.dart`,
//! `rendering/shifted_box.dart`, `rendering/custom_layout.dart`) and `Flow`
//! (`rendering/flow.dart`), plus `PositionedDirectional` and
//! `IgnoreBaseline`.

use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size, Transform as CoreTransform};

use crate::tree::{OpacityController, RetainedState, WidgetKind};
use crate::{Builder, Opacity, Positioned, Widget};

/// Flutter's `SingleChildLayoutDelegate`: sizes the box, constrains the
/// child and places it.
pub trait SingleChildLayoutDelegate: 'static {
    /// The box's size, then constrained to `constraints`. Defaults to the
    /// biggest size the constraints allow.
    fn get_size(&self, constraints: Constraints) -> Size {
        constraints.biggest()
    }

    /// The constraints the child lays out under. Defaults to the box's.
    fn get_constraints_for_child(&self, constraints: Constraints) -> Constraints {
        constraints
    }

    /// The child's offset within the box. Defaults to the origin.
    fn get_position_for_child(&self, _size: Size, _child_size: Size) -> Offset {
        Offset::ZERO
    }

    /// Whether replacing `old` with this delegate needs a new layout. A
    /// delegate of another type always relayouts.
    fn should_relayout(&self, old: &Self) -> bool
    where
        Self: Sized;
}

/// Flutter's `MultiChildLayoutDelegate`: lays out and places the
/// [`LayoutId`] children of a [`CustomMultiChildLayout`].
pub trait MultiChildLayoutDelegate: 'static {
    /// The box's size, then constrained to `constraints`. Defaults to the
    /// biggest size the constraints allow.
    fn get_size(&self, constraints: Constraints) -> Size {
        constraints.biggest()
    }

    /// Lays out every child through `children` and positions them.
    fn perform_layout(&self, size: Size, children: &mut MultiChildLayoutContext<'_>);

    /// Whether replacing `old` with this delegate needs a new layout. A
    /// delegate of another type always relayouts.
    fn should_relayout(&self, old: &Self) -> bool
    where
        Self: Sized;
}

/// Flutter's `FlowDelegate`: sizes the flow, constrains each child and
/// paints the children with transforms.
pub trait FlowDelegate: 'static {
    /// The flow's size, then constrained to `constraints`. Defaults to the
    /// biggest size the constraints allow.
    fn get_size(&self, constraints: Constraints) -> Size {
        constraints.biggest()
    }

    /// The constraints the child at `index` lays out under. Defaults to the
    /// flow's.
    fn get_constraints_for_child(&self, _index: usize, constraints: Constraints) -> Constraints {
        constraints
    }

    /// Paints children through [`FlowPaintingContext::paint_child`].
    /// Children it does not paint are neither drawn nor hit-tested.
    fn paint_children(&self, context: &mut FlowPaintingContext<'_>);

    /// Whether replacing `old` with this delegate needs a new layout.
    /// Defaults to false.
    fn should_relayout(&self, _old: &Self) -> bool
    where
        Self: Sized,
    {
        false
    }

    /// Whether replacing `old` with this delegate needs the children
    /// painted again.
    fn should_repaint(&self, old: &Self) -> bool
    where
        Self: Sized;
}

/// A delegate with the type-aware comparison Flutter makes when a widget
/// replaces it: `runtimeType` differs or `shouldRelayout` says so.
#[doc(hidden)]
pub struct DelegateCell<D: ?Sized> {
    delegate: Rc<D>,
    concrete: Rc<dyn Any>,
    changed: fn(&dyn Any, &dyn Any) -> bool,
}

impl<D: ?Sized> Clone for DelegateCell<D> {
    fn clone(&self) -> Self {
        Self {
            delegate: self.delegate.clone(),
            concrete: self.concrete.clone(),
            changed: self.changed,
        }
    }
}

impl<D: ?Sized> DelegateCell<D> {
    pub(crate) fn get(&self) -> &D {
        &self.delegate
    }
}

/// Equal when replacing one with the other needs no new layout.
impl<D: ?Sized> PartialEq for DelegateCell<D> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.concrete, &other.concrete)
            || !(self.changed)(&*self.concrete, &*other.concrete)
    }
}

fn single_changed<T: SingleChildLayoutDelegate>(new: &dyn Any, old: &dyn Any) -> bool {
    match (new.downcast_ref::<T>(), old.downcast_ref::<T>()) {
        (Some(new), Some(old)) => new.should_relayout(old),
        _ => true,
    }
}

fn multi_changed<T: MultiChildLayoutDelegate>(new: &dyn Any, old: &dyn Any) -> bool {
    match (new.downcast_ref::<T>(), old.downcast_ref::<T>()) {
        (Some(new), Some(old)) => new.should_relayout(old),
        _ => true,
    }
}

/// A flow repaints by laying out again, so either answer counts.
fn flow_changed<T: FlowDelegate>(new: &dyn Any, old: &dyn Any) -> bool {
    match (new.downcast_ref::<T>(), old.downcast_ref::<T>()) {
        (Some(new), Some(old)) => new.should_relayout(old) || new.should_repaint(old),
        _ => true,
    }
}

/// The delegate of a [`WidgetKind::CustomLayout`].
#[derive(Clone, PartialEq)]
#[doc(hidden)]
pub enum CustomLayoutDelegate {
    Single(DelegateCell<dyn SingleChildLayoutDelegate>),
    Multi(DelegateCell<dyn MultiChildLayoutDelegate>),
    Flow(DelegateCell<dyn FlowDelegate>),
}

impl fmt::Debug for CustomLayoutDelegate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.type_name())
    }
}

impl CustomLayoutDelegate {
    /// The Flutter widget this layout implements.
    pub(crate) const fn type_name(&self) -> &'static str {
        match self {
            Self::Single(_) => "CustomSingleChildLayout",
            Self::Multi(_) => "CustomMultiChildLayout",
            Self::Flow(_) => "Flow",
        }
    }
}

/// The id a [`LayoutId`] gives its child.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LayoutChildId(Rc<str>);

impl From<&str> for LayoutChildId {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for LayoutChildId {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<usize> for LayoutChildId {
    fn from(value: usize) -> Self {
        Self(value.to_string().into())
    }
}

/// What [`MultiChildLayoutDelegate::perform_layout`] lays its children out
/// through: Flutter's `hasChild`, `layoutChild` and `positionChild`.
pub struct MultiChildLayoutContext<'a> {
    ids: &'a HashMap<LayoutChildId, usize>,
    layout: &'a mut dyn FnMut(usize, Constraints) -> Size,
    offsets: &'a mut [Offset],
    laid_out: &'a mut [bool],
}

impl MultiChildLayoutContext<'_> {
    #[must_use]
    pub fn has_child(&self, id: impl Into<LayoutChildId>) -> bool {
        self.ids.contains_key(&id.into())
    }

    /// Lays the child out and returns its size.
    ///
    /// # Panics
    /// Panics when no child has `id`, where Flutter throws.
    pub fn layout_child(&mut self, id: impl Into<LayoutChildId>, constraints: Constraints) -> Size {
        let index = self.index(id.into());
        self.laid_out[index] = true;
        (self.layout)(index, constraints)
    }

    /// Places the child. Children start at the origin.
    ///
    /// # Panics
    /// Panics when no child has `id`, where Flutter throws.
    pub fn position_child(&mut self, id: impl Into<LayoutChildId>, offset: Offset) {
        let index = self.index(id.into());
        self.offsets[index] = offset;
    }

    fn index(&self, id: LayoutChildId) -> usize {
        *self
            .ids
            .get(&id)
            .unwrap_or_else(|| panic!("CustomMultiChildLayout has no child with id {id:?}"))
    }
}

/// Runs a multi-child delegate over children indexed by id. Children the
/// delegate does not lay out, which Flutter rejects, lay out at zero size.
pub(crate) fn perform_multi_child_layout(
    delegate: &dyn MultiChildLayoutDelegate,
    size: Size,
    ids: &HashMap<LayoutChildId, usize>,
    child_count: usize,
    layout: &mut dyn FnMut(usize, Constraints) -> Size,
) -> Vec<Offset> {
    let mut offsets = vec![Offset::ZERO; child_count];
    let mut laid_out = vec![false; child_count];
    delegate.perform_layout(
        size,
        &mut MultiChildLayoutContext {
            ids,
            layout,
            offsets: &mut offsets,
            laid_out: &mut laid_out,
        },
    );
    for (index, laid_out) in laid_out.into_iter().enumerate() {
        if !laid_out {
            layout(index, Constraints::tight(Size::ZERO));
        }
    }
    offsets
}

/// What [`FlowDelegate::paint_children`] paints through: Flutter's
/// `FlowPaintingContext`.
pub struct FlowPaintingContext<'a> {
    size: Size,
    child_sizes: &'a [Size],
    painted: Vec<Option<(CoreTransform, f32)>>,
}

impl<'a> FlowPaintingContext<'a> {
    pub(crate) fn new(size: Size, child_sizes: &'a [Size]) -> Self {
        Self {
            size,
            child_sizes,
            painted: vec![None; child_sizes.len()],
        }
    }

    /// The flow's size.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    #[must_use]
    pub const fn child_count(&self) -> usize {
        self.child_sizes.len()
    }

    #[must_use]
    pub fn get_child_size(&self, index: usize) -> Option<Size> {
        self.child_sizes.get(index).copied()
    }

    /// Paints the child at `index` with `transform` and `opacity`. Flutter
    /// paints children in the order of these calls; here children always
    /// paint in their own order.
    ///
    /// # Panics
    /// Panics when `index` is out of range or the child was already
    /// painted, as Flutter asserts.
    pub fn paint_child(&mut self, index: usize, transform: CoreTransform, opacity: f32) {
        let slot = &mut self.painted[index];
        assert!(slot.is_none(), "a flow child can be painted once");
        *slot = Some((transform, opacity.clamp(0., 1.)));
    }

    pub(crate) fn into_painted(self) -> Vec<Option<(CoreTransform, f32)>> {
        self.painted
    }
}

struct FlowSlotState {
    transform: Cell<CoreTransform>,
    painted: Cell<bool>,
    opacity: OpacityController,
}

/// One child's paint state in a [`Flow`], written by the flow's layout.
/// The slot wraps the child as a transform (and an opacity) whose value it
/// reads from here.
#[derive(Clone)]
#[doc(hidden)]
pub struct FlowSlot(Rc<FlowSlotState>);

impl PartialEq for FlowSlot {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for FlowSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FlowSlot")
            .field("transform", &self.0.transform.get())
            .field("painted", &self.0.painted.get())
            .finish()
    }
}

impl FlowSlot {
    fn new() -> Self {
        Self(Rc::new(FlowSlotState {
            transform: Cell::new(CoreTransform::IDENTITY),
            painted: Cell::new(false),
            opacity: OpacityController::new(),
        }))
    }

    pub(crate) fn transform(&self) -> CoreTransform {
        self.0.transform.get()
    }

    pub(crate) fn painted(&self) -> bool {
        self.0.painted.get()
    }

    pub(crate) fn opacity(&self) -> &OpacityController {
        &self.0.opacity
    }

    /// Records the delegate's paint call for this child, if any.
    pub(crate) fn set(&self, painted: Option<(CoreTransform, f32)>) {
        let (transform, opacity) = painted.unwrap_or((CoreTransform::IDENTITY, 1.));
        self.0.painted.set(painted.is_some());
        self.0.transform.set(transform);
        let _ = self.0.opacity.set_opacity(opacity);
    }

    fn wrap(&self, child: Widget) -> Widget {
        Widget::from_kind(WidgetKind::Transform {
            transform: CoreTransform::IDENTITY,
            origin: Some(Offset::ZERO),
            fraction: None,
            transform_hit_tests: true,
            slot: Some(self.clone()),
            child: Opacity::controlled(self.0.opacity.clone(), child).into(),
        })
    }
}

/// Flutter's `CustomSingleChildLayout`.
pub struct CustomSingleChildLayout {
    delegate: DelegateCell<dyn SingleChildLayoutDelegate>,
    child: Widget,
}

impl CustomSingleChildLayout {
    #[must_use]
    pub fn new<D: SingleChildLayoutDelegate>(delegate: D, child: impl Into<Widget>) -> Self {
        let delegate = Rc::new(delegate);
        Self {
            delegate: DelegateCell {
                delegate: delegate.clone(),
                concrete: delegate,
                changed: single_changed::<D>,
            },
            child: child.into(),
        }
    }
}

impl From<CustomSingleChildLayout> for Widget {
    fn from(value: CustomSingleChildLayout) -> Self {
        Widget::from_kind(WidgetKind::CustomLayout {
            delegate: CustomLayoutDelegate::Single(value.delegate),
            children: vec![value.child],
        })
    }
}

/// Flutter's `CustomMultiChildLayout`: every child is a [`LayoutId`].
pub struct CustomMultiChildLayout {
    delegate: DelegateCell<dyn MultiChildLayoutDelegate>,
    children: Vec<Widget>,
}

impl CustomMultiChildLayout {
    #[must_use]
    pub fn new<D: MultiChildLayoutDelegate>(
        delegate: D,
        children: impl IntoIterator<Item = impl Into<Widget>>,
    ) -> Self {
        let delegate = Rc::new(delegate);
        Self {
            delegate: DelegateCell {
                delegate: delegate.clone(),
                concrete: delegate,
                changed: multi_changed::<D>,
            },
            children: children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CustomMultiChildLayout> for Widget {
    fn from(value: CustomMultiChildLayout) -> Self {
        Widget::from_kind(WidgetKind::CustomLayout {
            delegate: CustomLayoutDelegate::Multi(value.delegate),
            children: value.children,
        })
    }
}

/// Flutter's `LayoutId`: names a [`CustomMultiChildLayout`] child.
pub struct LayoutId {
    id: LayoutChildId,
    child: Widget,
}

impl LayoutId {
    #[must_use]
    pub fn new(id: impl Into<LayoutChildId>, child: impl Into<Widget>) -> Self {
        Self {
            id: id.into(),
            child: child.into(),
        }
    }
}

impl From<LayoutId> for Widget {
    fn from(value: LayoutId) -> Self {
        Widget::from_kind(WidgetKind::LayoutId {
            id: value.id,
            child: value.child,
        })
    }
}

/// Flutter's `Flow`: lays each child out under the delegate's constraints
/// at the origin, then paints it with the transform and opacity the
/// delegate gives. Paint transforms are compositor-only, as in Flutter.
///
/// The delegate's paint runs as part of the flow's layout, so a flow that
/// should repaint, for example from an animation, needs a new delegate.
pub struct Flow {
    delegate: DelegateCell<dyn FlowDelegate>,
    children: Vec<Widget>,
}

impl Flow {
    #[must_use]
    pub fn new<D: FlowDelegate>(
        delegate: D,
        children: impl IntoIterator<Item = impl Into<Widget>>,
    ) -> Self {
        let delegate = Rc::new(delegate);
        Self {
            delegate: DelegateCell {
                delegate: delegate.clone(),
                concrete: delegate,
                changed: flow_changed::<D>,
            },
            children: children.into_iter().map(Into::into).collect(),
        }
    }
}

/// The flow's slots, kept across rebuilds so each child's paint state and
/// opacity controller stay attached to it.
struct FlowSlots(RefCell<Vec<FlowSlot>>);

impl RetainedState for FlowSlots {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl From<Flow> for Widget {
    fn from(value: Flow) -> Self {
        let value = Rc::new(value);
        Widget::retained_builder(
            |_| FlowSlots(RefCell::new(Vec::new())),
            move |slots, _, _| {
                let mut slots = slots.0.borrow_mut();
                while slots.len() < value.children.len() {
                    slots.push(FlowSlot::new());
                }
                Widget::from_kind(WidgetKind::CustomLayout {
                    delegate: CustomLayoutDelegate::Flow(value.delegate.clone()),
                    children: value
                        .children
                        .iter()
                        .zip(slots.iter())
                        .map(|(child, slot)| slot.wrap(child.clone()))
                        .collect(),
                })
            },
            false,
        )
    }
}

/// Flutter's `PositionedDirectional`: a [`Positioned`] whose start and end
/// follow the ambient [`Directionality`](crate::Directionality).
#[derive(Clone)]
pub struct PositionedDirectional {
    start: Option<f32>,
    top: Option<f32>,
    end: Option<f32>,
    bottom: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
    child: Widget,
}

impl PositionedDirectional {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            start: None,
            top: None,
            end: None,
            bottom: None,
            width: None,
            height: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn start(mut self, start: f32) -> Self {
        self.start = Some(start);
        self
    }

    #[must_use]
    pub fn top(mut self, top: f32) -> Self {
        self.top = Some(top);
        self
    }

    #[must_use]
    pub fn end(mut self, end: f32) -> Self {
        self.end = Some(end);
        self
    }

    #[must_use]
    pub fn bottom(mut self, bottom: f32) -> Self {
        self.bottom = Some(bottom);
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

impl From<PositionedDirectional> for Widget {
    fn from(value: PositionedDirectional) -> Self {
        Builder::new(move |context| {
            // `Positioned.directional`.
            let (left, right) = if context.text_direction() == Some(TextDirection::Rtl) {
                (value.end, value.start)
            } else {
                (value.start, value.end)
            };
            let mut positioned = Positioned::new(value.child.clone());
            for (edge, set) in [
                (left, Positioned::left as fn(Positioned, f32) -> Positioned),
                (value.top, Positioned::top),
                (right, Positioned::right),
                (value.bottom, Positioned::bottom),
                (value.width, Positioned::width),
                (value.height, Positioned::height),
            ] {
                if let Some(edge) = edge {
                    positioned = set(positioned, edge);
                }
            }
            positioned.into()
        })
        .into()
    }
}

/// Flutter's `IgnoreBaseline`: the child lays out unchanged but reports no
/// baseline, so baseline alignment treats it as having none.
pub struct IgnoreBaseline {
    child: Widget,
}

impl IgnoreBaseline {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl From<IgnoreBaseline> for Widget {
    fn from(value: IgnoreBaseline) -> Self {
        // A builder is its own render object, and only text reports a
        // baseline, so the child's never reaches the parent.
        Builder::new(move |_| value.child.clone()).into()
    }
}
