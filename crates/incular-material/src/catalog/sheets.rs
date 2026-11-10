//! Flutter's `showModalBottomSheet` (`material/bottom_sheet.dart`): a
//! sheet rising from the bottom over a scrim.

use std::{cell::Cell, rc::Rc};

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisSize};
use incular_core::Color;
use incular_widgets::{
    Align, Alignment, BorderRadius, Column, ConstrainedBox, Container, GestureDetector,
    OverlayPortal, Positioned, Radius, SizedBox, Stack, Widget,
};

use super::{Revision, theme_of};
use crate::material_theme::helpers::alpha;

/// A modal bottom sheet's presentation handle: open it, close it, and
/// present it over a subtree.
#[derive(Clone)]
pub struct ModalBottomSheet {
    builder: Rc<dyn Fn() -> Widget>,
    open: Rc<Cell<bool>>,
    revision: Revision,
    is_dismissible: bool,
    show_drag_handle: bool,
    background_color: Option<Color>,
    barrier_color: Option<Color>,
    max_width: f32,
    on_closed: Option<Rc<dyn Fn()>>,
}

impl ModalBottomSheet {
    /// `_BottomSheetDefaultsM3.constraints`: at most 640 wide.
    pub const MAX_WIDTH: f32 = 640.;

    #[must_use]
    pub fn new(builder: impl Fn() -> Widget + 'static) -> Self {
        Self {
            builder: Rc::new(builder),
            open: Rc::new(Cell::new(false)),
            revision: Revision::default(),
            is_dismissible: true,
            show_drag_handle: false,
            background_color: None,
            barrier_color: None,
            max_width: Self::MAX_WIDTH,
            on_closed: None,
        }
    }

    #[must_use]
    pub fn is_dismissible(mut self, dismissible: bool) -> Self {
        self.is_dismissible = dismissible;
        self
    }

    #[must_use]
    pub fn show_drag_handle(mut self, show: bool) -> Self {
        self.show_drag_handle = show;
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[must_use]
    pub fn barrier_color(mut self, color: Color) -> Self {
        self.barrier_color = Some(color);
        self
    }

    #[must_use]
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    #[must_use]
    pub fn on_closed(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_closed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.get()
    }

    pub fn open(&self) {
        if !self.open.replace(true) {
            self.revision.bump();
        }
    }

    pub fn close(&self) {
        if self.open.replace(false) {
            self.revision.bump();
            if let Some(callback) = &self.on_closed {
                callback();
            }
        }
    }

    fn sheet(&self, context: &incular_widgets::BuildContext<'_>) -> Widget {
        let theme = theme_of(context);
        let scheme = &theme.core().color_scheme;
        let mut content: Vec<Widget> = Vec::new();
        if self.show_drag_handle {
            // `_DragHandle`: 32x4 in a 48px target, `onSurfaceVariant` at 40%.
            content.push(
                SizedBox::new()
                    .height(48.)
                    .child(incular_widgets::Center::new(
                        Container::new().width(32.).height(4.).decoration(
                            incular_widgets::BoxDecoration::new()
                                .color(alpha(scheme.on_surface_variant, 0.4))
                                .border_radius(BorderRadius::circular(2.)),
                        ),
                    ))
                    .into(),
            );
        }
        content.push((self.builder)());
        let radius = Radius::circular(28.);
        let sheet: Widget = crate::Material::new(
            Column::new(content)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch),
        )
        .color(
            self.background_color
                .unwrap_or(scheme.surface_container_low),
        )
        .elevation(1.)
        .border_radius(BorderRadius::vertical(radius, Radius::circular(0.)))
        .into();
        // Taps on the sheet itself must not reach the dismissing scrim.
        let sheet: Widget = GestureDetector::new(sheet).on_tap(|| {}).into();
        let dismiss = self.clone();
        let barrier: Widget = GestureDetector::new(
            Container::new().color(self.barrier_color.unwrap_or(Color::rgba(0, 0, 0, 138))),
        )
        .on_tap(move || {
            if dismiss.is_dismissible {
                dismiss.close();
            }
        })
        .into();
        Stack::new([
            Widget::from(Positioned::fill(barrier)),
            Widget::from(Positioned::fill(Align::new(
                Alignment::BOTTOM_CENTER,
                ConstrainedBox::new(
                    incular_config::Constraints::new(0., self.max_width, 0., f32::INFINITY),
                    Container::new().padding(EdgeInsets::zero()).child(sheet),
                ),
            ))),
        ])
        .into()
    }

    /// Presents the sheet over `child` while it is open.
    #[must_use]
    pub fn present(&self, child: impl Into<Widget>) -> Widget {
        let child = child.into();
        let sheet = self.clone();
        Widget::stateful_builder(self.revision.cell(), move |context| {
            let open = sheet.is_open();
            let mut portal = OverlayPortal::new(child.clone()).show(open);
            if open {
                portal = portal.overlay_child(sheet.sheet(context));
            }
            let widget: Widget = portal.into();
            if open {
                widget.block_semantics()
            } else {
                widget
            }
        })
    }
}

/// Flutter's `showModalBottomSheet`: a sheet built by `builder`, opened at
/// once. Present it with [`ModalBottomSheet::present`].
#[must_use]
pub fn show_modal_bottom_sheet(builder: impl Fn() -> Widget + 'static) -> ModalBottomSheet {
    let sheet = ModalBottomSheet::new(builder);
    sheet.open();
    sheet
}
