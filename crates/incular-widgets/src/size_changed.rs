//! Flutter's `SizeChangedLayoutNotifier`
//! (`widgets/size_changed_layout_notifier.dart`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::Size;

use crate::Widget;
use crate::tree::RetainedState;

/// Flutter's `SizeChangedLayoutNotifier`: reports each layout in which the
/// child's size differs from its previous size, but not the first.
///
/// Flutter dispatches a `SizeChangedLayoutNotification` to ancestor
/// listeners; Incular has no general notification bubbling, so the change
/// goes to `on_changed` with the new size. It runs during layout.
pub struct SizeChangedLayoutNotifier {
    on_changed: Rc<dyn Fn(Size)>,
    child: Widget,
}

impl SizeChangedLayoutNotifier {
    #[must_use]
    pub fn new(on_changed: impl Fn(Size) + 'static, child: impl Into<Widget>) -> Self {
        Self {
            on_changed: Rc::new(on_changed),
            child: child.into(),
        }
    }
}

/// `_RenderSizeChangedWithCallback._oldSize`.
struct LastSize(Rc<Cell<Option<Size>>>);

impl RetainedState for LastSize {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl From<SizeChangedLayoutNotifier> for Widget {
    fn from(value: SizeChangedLayoutNotifier) -> Self {
        let value = RefCell::new(value);
        Widget::retained_builder(
            |_| LastSize(Rc::new(Cell::new(None))),
            move |state, _, _| {
                let value = value.borrow();
                let last = state.0.clone();
                let on_changed = value.on_changed.clone();
                Widget::size_observer(
                    move |size| {
                        if last.replace(Some(size)).is_some_and(|old| old != size) {
                            on_changed(size);
                        }
                    },
                    value.child.clone(),
                )
            },
            false,
        )
    }
}
