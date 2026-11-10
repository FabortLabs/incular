//! Flutter's `CarouselView` and `CarouselController`
//! (`material/carousel.dart`): a horizontally scrolling list of clipped,
//! tappable items of a fixed extent.

use std::rc::Rc;

use incular_config::{Axis, EdgeInsets};
use incular_core::Color;
use incular_widgets::{
    ClipRRect, Container, ListView, ScrollController, SizedBox, Widget, internal::ActionSurface,
};

use super::theme_of;
use crate::material_theme::helpers::alpha;

/// Flutter's `CarouselController`: a scroll controller that moves by items.
#[derive(Clone, Default)]
pub struct CarouselController {
    scroll: ScrollController,
    item_extent: Rc<std::cell::Cell<f32>>,
}

impl CarouselController {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn scroll_controller(&self) -> &ScrollController {
        &self.scroll
    }

    /// Shows the item at `index` at the leading edge.
    pub fn jump_to_item(&self, index: usize) -> bool {
        self.scroll.jump_to(index as f32 * self.item_extent.get())
    }
}

/// Flutter's `CarouselView`: items `item_extent` wide, each padded by 4 and
/// clipped to a 28 radius, the Material 3 defaults.
#[derive(Clone)]
pub struct CarouselView {
    children: Vec<Widget>,
    item_extent: f32,
    shrink_extent: f32,
    padding: EdgeInsets,
    background_color: Option<Color>,
    radius: f32,
    item_snapping: bool,
    scroll_direction: Axis,
    controller: CarouselController,
    on_tap: Option<Rc<dyn Fn(usize)>>,
}

impl CarouselView {
    #[must_use]
    pub fn new(item_extent: f32, children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            item_extent: item_extent.max(0.),
            shrink_extent: 0.,
            padding: EdgeInsets::all(4.),
            background_color: None,
            radius: 28.,
            item_snapping: false,
            scroll_direction: Axis::Horizontal,
            controller: CarouselController::new(),
            on_tap: None,
        }
    }

    /// The smallest extent an item shrinks to at the edges. Incular keeps
    /// every item at `item_extent`; the value is recorded for parity.
    #[must_use]
    pub fn shrink_extent(mut self, extent: f32) -> Self {
        self.shrink_extent = extent.max(0.);
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[must_use]
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    #[must_use]
    pub fn item_snapping(mut self, snapping: bool) -> Self {
        self.item_snapping = snapping;
        self
    }

    #[must_use]
    pub fn scroll_direction(mut self, direction: Axis) -> Self {
        self.scroll_direction = direction;
        self
    }

    #[must_use]
    pub fn controller(mut self, controller: CarouselController) -> Self {
        self.controller = controller;
        self
    }

    #[must_use]
    pub fn on_tap(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self
    }
}

impl From<CarouselView> for Widget {
    fn from(view: CarouselView) -> Self {
        view.controller.item_extent.set(view.item_extent);
        let view = Rc::new(view);
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let background = view.background_color.unwrap_or(scheme.surface);
            let horizontal = view.scroll_direction == Axis::Horizontal;
            let items = view.children.iter().enumerate().map(|(index, child)| {
                let mut surface = ActionSurface::with_child(
                    Container::new().color(background).child(child.clone()),
                )
                .color(Color::TRANSPARENT)
                .hover_color(alpha(scheme.on_surface, 0.08))
                .pressed_color(alpha(scheme.on_surface, 0.1))
                .focused_color(alpha(scheme.on_surface, 0.1));
                if let Some(on_tap) = view.on_tap.clone() {
                    surface = surface.on_click(move || on_tap(index));
                }
                let item: Widget = Container::new()
                    .padding(view.padding)
                    .child(ClipRRect::new(view.radius, surface))
                    .into();
                let sized = if horizontal {
                    SizedBox::new().width(view.item_extent).child(item)
                } else {
                    SizedBox::new().height(view.item_extent).child(item)
                };
                Widget::from(sized)
            });
            let mut list = ListView::new(items.collect::<Vec<_>>())
                .scroll_direction(view.scroll_direction)
                .controller(view.controller.scroll.clone());
            if view.item_snapping {
                list = list.physics(incular_scroll::ScrollPhysics::clamping());
            }
            list.into()
        })
        .into()
    }
}
