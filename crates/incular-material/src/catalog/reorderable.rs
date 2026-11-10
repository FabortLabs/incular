//! Flutter's `ReorderableListView` (`material/reorderable_list.dart`): a
//! [`ReorderableList`] whose items get drag handles.

use std::rc::Rc;

use incular_config::{Axis, CrossAxisAlignment, EdgeInsets};
use incular_widgets::{
    Container, Expanded, ReorderableDelayedDragStartListener, ReorderableDragStartListener,
    ReorderableList, Row, Stack, Text, Widget,
};

use super::theme_of;

/// `Icons.drag_handle`.
const DRAG_HANDLE: &str = "≡";

/// Flutter's `ReorderableListView.builder`: on desktop each item gets a
/// trailing drag handle (`buildDefaultDragHandles`); elsewhere a long press
/// anywhere on the item starts the drag.
#[derive(Clone)]
pub struct ReorderableListView {
    item_count: usize,
    builder: Rc<dyn Fn(usize) -> Widget>,
    on_reorder: Option<Rc<dyn Fn(usize, usize)>>,
    build_default_drag_handles: bool,
    scroll_direction: Axis,
    padding: Option<EdgeInsets>,
    header: Option<Widget>,
}

impl ReorderableListView {
    #[must_use]
    pub fn builder<W: Into<Widget>>(
        item_count: usize,
        builder: impl Fn(usize) -> W + 'static,
    ) -> Self {
        Self {
            item_count,
            builder: Rc::new(move |index| builder(index).into()),
            on_reorder: None,
            build_default_drag_handles: true,
            scroll_direction: Axis::Vertical,
            padding: None,
            header: None,
        }
    }

    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        let children = children
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Widget>>();
        let count = children.len();
        Self::builder(count, move |index| children[index].clone())
    }

    /// Flutter's `onReorder(oldIndex, newIndex)`: `new_index` counts the
    /// moved item's old slot, as in Flutter.
    #[must_use]
    pub fn on_reorder(mut self, callback: impl Fn(usize, usize) + 'static) -> Self {
        self.on_reorder = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn build_default_drag_handles(mut self, build: bool) -> Self {
        self.build_default_drag_handles = build;
        self
    }

    #[must_use]
    pub fn scroll_direction(mut self, direction: Axis) -> Self {
        self.scroll_direction = direction;
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    #[must_use]
    pub fn header(mut self, header: impl Into<Widget>) -> Self {
        self.header = Some(header.into());
        self
    }
}

impl From<ReorderableListView> for Widget {
    fn from(view: ReorderableListView) -> Self {
        let view = Rc::new(view);
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let desktop = matches!(
                theme.core().platform,
                crate::TargetPlatform::Linux
                    | crate::TargetPlatform::MacOs
                    | crate::TargetPlatform::Windows
            );
            let handle_color = theme.core().color_scheme.on_surface_variant;
            let item_view = view.clone();
            let mut list = ReorderableList::builder(view.item_count, move |index| {
                let item = (item_view.builder)(index);
                if !item_view.build_default_drag_handles {
                    return item;
                }
                if desktop {
                    let handle: Widget = ReorderableDragStartListener::new(
                        index,
                        Container::new()
                            .padding(EdgeInsets::symmetric(8., 0.))
                            .child(Text::new(DRAG_HANDLE).color(handle_color)),
                    )
                    .into();
                    Row::new([Widget::from(Expanded::new(item)), handle])
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .into()
                } else {
                    ReorderableDelayedDragStartListener::new(index, item).into()
                }
            })
            .scroll_direction(view.scroll_direction);
            if let Some(callback) = view.on_reorder.clone() {
                list = list.on_reorder(move |old, new| callback(old, new));
            }
            if let Some(padding) = view.padding {
                list = list.padding(padding);
            }
            match &view.header {
                Some(header) => incular_widgets::Column::new([
                    header.clone(),
                    Widget::from(Expanded::new(list)),
                ])
                .into(),
                None => Stack::new([Widget::from(list)]).into(),
            }
        })
        .into()
    }
}
