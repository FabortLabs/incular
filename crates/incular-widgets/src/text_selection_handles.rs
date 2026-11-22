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

/// One handle of a text field's selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionHandle {
    /// Which edge or caret the handle marks.
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
        let tip = match self.handle_type {
            SelectionHandleType::Collapsed => Offset::new(SELECTION_HANDLE_SIZE / 2.0, -4.0),
            SelectionHandleType::Left => Offset::new(SELECTION_HANDLE_SIZE, 0.0),
            SelectionHandleType::Right => Offset::new(0.0, 0.0),
        };
        Rect::from_origin_size(
            self.anchor - tip,
            Size::new(SELECTION_HANDLE_SIZE, SELECTION_HANDLE_SIZE),
        )
    }
}

/// The handles of a text field's selection: a collapsed caret handle, or a
/// start and an end handle for a range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionHandles {
    /// The collapsed handle, or the range's start handle.
    pub start: TextSelectionHandle,
    /// The range's end handle; `None` when the selection is collapsed.
    pub end: Option<TextSelectionHandle>,
}

impl TextSelectionHandles {
    /// The handle under `point` in global coordinates, or `None`. This is the
    /// seam a drag of a visible handle hit-tests through.
    #[must_use]
    pub fn hit_test(&self, point: Offset) -> Option<SelectionHandleType> {
        if self.start.rect().contains(point) {
            return Some(self.start.handle_type);
        }
        self.end
            .filter(|end| end.rect().contains(point))
            .map(|end| end.handle_type)
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
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for TextSelectionHandlesState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl TextSelectionHandlesState {
    fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            handles: RefCell::new(None),
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
        self.handles()?.hit_test(point)
    }
}

/// Lays `child` out unchanged and keeps a zero-size portal beside it that
/// presents the handles while `state` shows them. As with the context menu,
/// the child keeps its element when the handles toggle.
pub(crate) fn with_text_selection_handles(
    child: impl Fn(Rc<TextSelectionHandlesState>) -> Widget + 'static,
    builder: TextSelectionHandlesBuilder,
) -> Widget {
    Widget::retained_builder(
        TextSelectionHandlesState::new,
        move |state, _, _| {
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
