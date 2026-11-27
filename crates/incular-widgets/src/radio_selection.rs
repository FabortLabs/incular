//! Radio group and rich selection infrastructure widgets.

use crate::selection::SelectionAreaController;
use crate::{
    BuildContext, EditableTextContextMenuBuilder, GestureDetector, Text, TextContextMenu, Widget,
};
use std::{fmt, rc::Rc};

/// Coordinates mutually exclusive selection for a group of radio buttons.
#[derive(Clone)]
pub struct RadioGroup<T: Clone + PartialEq + 'static> {
    value: Option<T>,
    on_changed: Option<Rc<dyn Fn(T)>>,
    child: Widget,
}

impl<T: Clone + PartialEq + 'static> RadioGroup<T> {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            value: None,
            on_changed: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn value(mut self, value: Option<T>) -> Self {
        self.value = value;
        self
    }

    #[must_use]
    pub fn on_changed(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_changed = Some(Rc::new(callback));
        self
    }
}

impl<T: Clone + PartialEq + 'static> From<RadioGroup<T>> for Widget {
    fn from(value: RadioGroup<T>) -> Self {
        value.child
    }
}

/// A raw radio button control.
#[derive(Clone)]
pub struct RawRadio<T: Clone + PartialEq + 'static> {
    value: T,
    group_value: Option<T>,
    on_changed: Option<Rc<dyn Fn(T)>>,
}

impl<T: Clone + PartialEq + 'static> RawRadio<T> {
    #[must_use]
    pub fn new(value: T, group_value: Option<T>) -> Self {
        Self {
            value,
            group_value,
            on_changed: None,
        }
    }

    #[must_use]
    pub fn on_changed(mut self, callback: impl Fn(T) + 'static) -> Self {
        self.on_changed = Some(Rc::new(callback));
        self
    }
}

impl<T: Clone + PartialEq + 'static> From<RawRadio<T>> for Widget {
    fn from(value: RawRadio<T>) -> Self {
        let is_selected = value.group_value.as_ref() == Some(&value.value);
        let radio_symbol = if is_selected { "(•)" } else { "( )" };
        let item_val = value.value;
        let mut gd = GestureDetector::new(Text::new(radio_symbol));
        if let Some(cb) = value.on_changed {
            gd = gd.on_tap(move || cb(item_val.clone()));
        }
        gd.into()
    }
}

/// An area that supports pointer-based text selection.
#[derive(Clone)]
pub struct SelectableRegion {
    child: Widget,
    context_menu_builder: Option<EditableTextContextMenuBuilder>,
}

impl fmt::Debug for SelectableRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectableRegion")
            .field("child", &self.child)
            .field(
                "has_context_menu_builder",
                &self.context_menu_builder.is_some(),
            )
            .finish()
    }
}

impl PartialEq for SelectableRegion {
    fn eq(&self, other: &Self) -> bool {
        self.child == other.child
            && match (&self.context_menu_builder, &other.context_menu_builder) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}

impl SelectableRegion {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            context_menu_builder: None,
        }
    }

    /// Builds the selection toolbar at the supplied anchors. The menu model
    /// contains the platform's ordered actions without prescribing a style.
    #[must_use]
    pub fn context_menu_builder(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, &TextContextMenu) -> Widget + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(builder));
        self
    }
}

impl From<SelectableRegion> for Widget {
    fn from(value: SelectableRegion) -> Self {
        let controller = SelectionAreaController::new();
        match value.context_menu_builder {
            Some(builder) => {
                Widget::selection_area_with_context_menu(controller, value.child, builder)
            }
            None => Widget::selection_area(controller, value.child),
        }
    }
}
