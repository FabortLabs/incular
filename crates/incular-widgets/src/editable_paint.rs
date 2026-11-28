//! Painter hooks in the editor's own viewport coordinate space.

use std::{any::Any, fmt, rc::Rc};

use incular_core::{Offset, Rect, Size, reactivity::DependencySource};
use incular_rendering::DisplayList;
use incular_text::TextSelection;

/// Geometry of a mounted editor when its picture is recorded. The supplied
/// display list uses local viewport coordinates, not the scrolled text origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditableTextPaintContext {
    pub size: Size,
    pub text_origin: Offset,
    pub scroll_offset: Offset,
    /// The local caret rectangle, whether or not the caret is currently visible.
    pub caret: Rect,
    pub selection: Option<TextSelection>,
    pub focused: bool,
}

/// Paints auxiliary content behind or over an editor's text and decorations.
pub trait EditableTextPainter {
    fn paint(&self, context: EditableTextPaintContext, canvas: &mut DisplayList);

    /// A false result retains the existing picture until the next repaint.
    fn should_repaint(&self, _previous: Option<&Self>) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Optional signal that requests a paint without rebuilding the editor.
    fn repaint_source(&self) -> Option<DependencySource> {
        None
    }
}

type Record = fn(&dyn Any, EditableTextPaintContext, &mut DisplayList);
type ShouldRepaint = fn(&dyn Any, Option<&dyn Any>) -> bool;

#[derive(Clone)]
pub struct EditableTextPainterDelegate {
    painter: Rc<dyn Any>,
    record: Record,
    should_repaint: ShouldRepaint,
    source: Option<DependencySource>,
}

impl EditableTextPainterDelegate {
    #[must_use]
    pub fn new<P: EditableTextPainter + 'static>(painter: P) -> Self {
        Self {
            source: painter.repaint_source(),
            painter: Rc::new(painter),
            record: |painter, context, canvas| {
                painter
                    .downcast_ref::<P>()
                    .expect("retained editable painter type")
                    .paint(context, canvas);
            },
            should_repaint: |painter, old| {
                let painter = painter
                    .downcast_ref::<P>()
                    .expect("retained editable painter type");
                let previous = old.and_then(|old| old.downcast_ref::<P>());
                (old.is_some() && previous.is_none()) || painter.should_repaint(previous)
            },
        }
    }

    pub(crate) fn record(&self, context: EditableTextPaintContext, canvas: &mut DisplayList) {
        (self.record)(&*self.painter, context, canvas);
    }

    pub(crate) fn source(&self) -> Option<DependencySource> {
        self.source.clone()
    }
}

impl fmt::Debug for EditableTextPainterDelegate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditableTextPainterDelegate")
            .finish_non_exhaustive()
    }
}

impl PartialEq for EditableTextPainterDelegate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.painter, &other.painter)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EditorPainters {
    pub(crate) background: Option<EditableTextPainterDelegate>,
    pub(crate) foreground: Option<EditableTextPainterDelegate>,
}

pub(crate) fn painters_changed(old: Option<&EditorPainters>, new: Option<&EditorPainters>) -> bool {
    fn changed(
        old: Option<&EditableTextPainterDelegate>,
        new: Option<&EditableTextPainterDelegate>,
    ) -> bool {
        if old == new {
            return false;
        }
        match new {
            Some(new) => (new.should_repaint)(&*new.painter, old.map(|value| &*value.painter)),
            // Removing a painter can leave old pixels behind.
            None => true,
        }
    }
    changed(
        old.and_then(|p| p.background.as_ref()),
        new.and_then(|p| p.background.as_ref()),
    ) || changed(
        old.and_then(|p| p.foreground.as_ref()),
        new.and_then(|p| p.foreground.as_ref()),
    )
}

/// Ranges occupied by the two auxiliary passes in the last local picture.
/// A signal can replace one pass without invoking the editor's own paint.
#[derive(Clone, Debug)]
pub(crate) struct EditorPaintRecording {
    pub(crate) context: EditableTextPaintContext,
    pub(crate) slots: [std::ops::Range<usize>; 2],
    pub(crate) isolated_clip: Option<usize>,
    pub(crate) pending: u8,
}
