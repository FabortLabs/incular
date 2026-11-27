//! Flutter's text selection handles for editable text: the teardrop
//! endpoints that a touch selection shows at the ends of the caret or the
//! selection (`widgets/text_selection.dart` `TextSelectionHandleType` and
//! `material/text_selection.dart` `getHandleAnchor`).
//!
//! The widgets layer decides *when* handles show and *where* their endpoints
//! sit. How a handle looks is the builder's job: Material paints a teardrop,
//! and a bare [`EditableText`] (like Flutter's) installs none.
//!
//! [`EditableText`]: crate::EditableText

use crate::{
    BuildContext, OverlayPortal, Positioned, SelectionHandleType, SizedBox, Stack, StackFit,
    TransientRole, Widget, tree::RetainedState,
};
use incular_config::{Clip, TextDirection};
use incular_core::{Offset, Rect, Size};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Flutter `material/text_selection.dart` `_kHandleSize`: the side of the
/// square every handle is laid out in.
pub const SELECTION_HANDLE_SIZE: f32 = 22.0;

/// Layout of a selection handle relative to the caret endpoint it marks.
/// The anchor is measured from the handle's top-left; it may be outside the
/// visible box when the visual handle sits clear of the text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionHandleLayout {
    pub size: Size,
    pub anchor: Offset,
}

impl TextSelectionHandleLayout {
    #[must_use]
    pub const fn new(size: Size, anchor: Offset) -> Self {
        Self { size, anchor }
    }

    fn standard(handle_type: SelectionHandleType) -> Self {
        let anchor = match handle_type {
            SelectionHandleType::Collapsed => Offset::new(SELECTION_HANDLE_SIZE / 2., -4.),
            SelectionHandleType::Left => Offset::new(SELECTION_HANDLE_SIZE, 0.),
            SelectionHandleType::Right => Offset::ZERO,
        };
        Self::new(
            Size::new(SELECTION_HANDLE_SIZE, SELECTION_HANDLE_SIZE),
            anchor,
        )
    }

    #[must_use]
    pub fn rect(self, endpoint: Offset) -> Rect {
        Rect::from_origin_size(endpoint - self.anchor, self.size)
    }

    /// Enlarges the visual handle around its center to the minimum
    /// interactive dimensions, without moving its tip.
    #[must_use]
    pub fn interactive_rect(self, endpoint: Offset) -> Rect {
        let rect = self.rect(endpoint);
        let width = self.size.width.max(MIN_INTERACTIVE_DIMENSION);
        let height = self.size.height.max(MIN_INTERACTIVE_DIMENSION);
        Rect::from_origin_size(
            rect.origin
                - Offset::new(
                    (width - rect.size.width) * 0.5,
                    (height - rect.size.height) * 0.5,
                ),
            Size::new(width, height),
        )
    }

    fn clearance_below_endpoint(self) -> f32 {
        self.size.height * 0.5 - self.anchor.y
            + self.size.height.max(MIN_INTERACTIVE_DIMENSION) * 0.5
    }
}

/// One handle of a text field's selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionHandle {
    /// The artwork to draw, independent of the boundary a drag moves.
    pub handle_type: SelectionHandleType,
    /// The endpoint the handle's tip touches: the bottom of the endpoint's
    /// caret line, in the field's global coordinates.
    pub anchor: Offset,
    /// Height of the caret line the endpoint sits on.
    pub line_height: f32,
    /// Direction of the text at the endpoint.
    pub direction: TextDirection,
}

impl TextSelectionHandle {
    /// The handle's box in global coordinates. Offsets follow Flutter's
    /// `MaterialTextSelectionHandleControls.getHandleAnchor`, which places
    /// the tip of each teardrop on [`anchor`](Self::anchor).
    #[must_use]
    pub fn rect(&self) -> Rect {
        TextSelectionHandleLayout::standard(self.handle_type).rect(self.anchor)
    }

    /// The area a touch on the handle is recognised in: Flutter
    /// `_SelectionHandleOverlay.build`, which expands the handle's rect to
    /// include a circle of `kMinInteractiveDimension` diameter around its
    /// centre. The result is a square of that side, centred on the handle.
    #[must_use]
    pub fn interactive_rect(&self) -> Rect {
        TextSelectionHandleLayout::standard(self.handle_type).interactive_rect(self.anchor)
    }
}

/// Flutter `kMinInteractiveDimension`: the smallest touch target a handle
/// gets, in logical pixels.
pub const MIN_INTERACTIVE_DIMENSION: f32 = 48.0;

/// The handles of a text field's selection: a collapsed caret handle, or a
/// start and an end handle for a range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionHandles {
    /// The collapsed handle, or the range's start handle.
    pub start: TextSelectionHandle,
    /// The range's end handle; `None` when the selection is collapsed.
    pub end: Option<TextSelectionHandle>,
}

/// The logical part of the selection targeted by a handle. Its artwork
/// can point left or right independently of which boundary the drag moves.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSelectionHandleEndpoint {
    Collapsed,
    Start,
    End,
}

impl TextSelectionHandles {
    #[doc(hidden)]
    #[must_use]
    pub fn endpoint(&self, endpoint: TextSelectionHandleEndpoint) -> Option<TextSelectionHandle> {
        match endpoint {
            TextSelectionHandleEndpoint::Collapsed | TextSelectionHandleEndpoint::Start => Some(self.start),
            TextSelectionHandleEndpoint::End => self.end,
        }
    }

    /// The handle under `point` in global coordinates, or `None`. A point
    /// counts when it lies in the handle's [`interactive_rect`]; when both
    /// handles' areas cover it, the handle whose tip is nearer wins.
    ///
    /// [`interactive_rect`]: TextSelectionHandle::interactive_rect
    #[must_use]
    pub fn hit_test(&self, point: Offset) -> Option<SelectionHandleType> {
        self.hit_test_endpoint(point)
            .and_then(|endpoint| self.endpoint(endpoint))
            .map(|handle| handle.handle_type)
    }

    fn hit_test_endpoint(&self, point: Offset) -> Option<TextSelectionHandleEndpoint> {
        use TextSelectionHandleEndpoint::{Collapsed, End, Start};
        let start_endpoint = if self.end.is_none() && self.start.handle_type == SelectionHandleType::Collapsed {
            Collapsed
        } else {
            Start
        };
        let start = self
            .start
            .interactive_rect()
            .contains(point)
            .then_some(self.start);
        let end = self
            .end
            .filter(|end| end.interactive_rect().contains(point));
        match (start, end) {
            (Some(start), Some(end)) => {
                let distance = |handle: &TextSelectionHandle| {
                    let delta = point - handle.anchor;
                    delta.x * delta.x + delta.y * delta.y
                };
                if distance(&end) < distance(&start) {
                    Some(End)
                } else {
                    Some(start_endpoint)
                }
            }
            (Some(_), None) => Some(start_endpoint),
            (None, end) => end.map(|_| End),
        }
    }
}

/// Builds the handles overlay. The returned widget fills the overlay and
/// places each handle from [`TextSelectionHandles`], in global coordinates.
pub type TextSelectionHandlesBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, &TextSelectionHandles) -> Widget>;

/// The visible handles of one editable text element. The runtime shows them
/// after a touch selection and keeps them on the selection's endpoints.
pub struct TextSelectionHandlesState {
    handles: RefCell<Option<TextSelectionHandles>>,
    collapsed_layout: Cell<TextSelectionHandleLayout>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for TextSelectionHandlesState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl TextSelectionHandlesState {
    fn new(revision: Rc<Cell<u64>>, collapsed_layout: TextSelectionHandleLayout) -> Self {
        Self {
            handles: RefCell::new(None),
            collapsed_layout: Cell::new(collapsed_layout),
            revision: RefCell::new(revision),
        }
    }

    fn bump(&self) {
        let revision = self.revision.borrow().clone();
        revision.set(revision.get().wrapping_add(1));
    }

    /// Whether handles are showing.
    #[must_use]
    pub fn is_showing(&self) -> bool {
        self.handles.borrow().is_some()
    }

    /// Shows `handles`, replacing any showing handles.
    pub fn show(&self, handles: TextSelectionHandles) {
        *self.handles.borrow_mut() = Some(handles);
        self.bump();
    }

    /// Moves the showing handles to `handles` when they differ. Does nothing
    /// while hidden. Returns whether the layout changed, so a caller only
    /// schedules a rebuild on a real change.
    pub fn update(&self, handles: TextSelectionHandles) -> bool {
        let mut current = self.handles.borrow_mut();
        match current.as_mut() {
            Some(showing) if *showing != handles => {
                *showing = handles;
                drop(current);
                self.bump();
                true
            }
            _ => false,
        }
    }

    /// Hides the handles. Returns whether they were showing.
    pub fn hide(&self) -> bool {
        let was_showing = self.handles.borrow_mut().take().is_some();
        if was_showing {
            self.bump();
        }
        was_showing
    }

    /// The showing handles.
    #[must_use]
    pub fn handles(&self) -> Option<TextSelectionHandles> {
        *self.handles.borrow()
    }

    /// The handle under `point` among the showing handles.
    #[must_use]
    pub fn hit_test(&self, point: Offset) -> Option<SelectionHandleType> {
        let endpoint = self.hit_test_endpoint(point)?;
        self.handles()?.endpoint(endpoint).map(|handle| handle.handle_type)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn hit_test_endpoint(&self, point: Offset) -> Option<TextSelectionHandleEndpoint> {
        let handles = self.handles()?;
        if handles.end.is_none() && handles.start.handle_type == SelectionHandleType::Collapsed {
            return self
                .collapsed_layout
                .get()
                .interactive_rect(handles.start.anchor)
                .contains(point)
                .then_some(TextSelectionHandleEndpoint::Collapsed);
        }
        handles.hit_test_endpoint(point)
    }

    pub(crate) fn caret_reveal_bottom_clearance(&self) -> f32 {
        self.collapsed_layout.get().clearance_below_endpoint()
    }
}

/// Lays `child` out unchanged and keeps a zero-size portal beside it that
/// presents the handles while `state` shows them. As with the context menu,
/// the child keeps its element when the handles toggle.
pub(crate) fn with_text_selection_handles(
    child: impl Fn(Rc<TextSelectionHandlesState>) -> Widget + 'static,
    builder: TextSelectionHandlesBuilder,
    collapsed_layout: Option<TextSelectionHandleLayout>,
) -> Widget {
    let collapsed_layout = collapsed_layout
        .unwrap_or_else(|| TextSelectionHandleLayout::standard(SelectionHandleType::Collapsed));
    Widget::retained_builder(
        move |revision| TextSelectionHandlesState::new(revision, collapsed_layout),
        move |state, _, _| {
            state.collapsed_layout.set(collapsed_layout);
            let handles = state.handles();
            let mut portal = OverlayPortal::new(SizedBox::shrink())
                .role(TransientRole::ContextMenu)
                .show(handles.is_some());
            if let Some(handles) = handles {
                let builder = builder.clone();
                // Flutter `_SelectionToolbarWrapper`: a press on a handle is
                // not a tap outside the field, which keeps its focus.
                portal = portal.overlay_child_layout_builder(move |context, _| {
                    crate::TextFieldTapRegion::new(builder(context, &handles)).into()
                });
            }
            Stack::new([
                child(state.clone()),
                Positioned::builder()
                    .left(0.0)
                    .top(0.0)
                    .width(0.0)
                    .height(0.0)
                    .child(portal)
                    .build()
                    .into(),
            ])
            .fit(StackFit::Passthrough)
            .clip_behavior(Clip::None)
            .into()
        },
        false,
    )
}
