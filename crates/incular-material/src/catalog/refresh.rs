//! Flutter's `RefreshIndicator` (`material/refresh_indicator.dart`): pull a
//! scrollable past its leading edge to refresh.

use std::{cell::Cell, rc::Rc};

use incular_core::Color;
use incular_scroll::{ScrollNotification, ScrollNotificationType};
use incular_widgets::{
    Align, Alignment, BoxDecoration, BoxShape, Container, NotificationListener, Positioned,
    SizedBox, Stack, Widget,
};

use super::{Revision, theme_of};
use crate::{CircularProgressIndicator, RefreshIndicatorStatus, RefreshIndicatorTriggerMode};

/// `_kDragContainerExtentPercentage`.
const DRAG_CONTAINER_EXTENT_PERCENTAGE: f32 = 0.25;
/// `_kIndicatorSize`-ish: the 41px circular indicator surface.
const INDICATOR_SIZE: f32 = 41.;

/// Ends a refresh started by a [`RefreshIndicator`] (Flutter's `onRefresh`
/// future completing).
#[derive(Clone)]
pub struct RefreshCompleter {
    state: Rc<RefreshState>,
}

impl RefreshCompleter {
    pub fn complete(&self) {
        self.state.set_mode(RefreshIndicatorStatus::Done);
        self.state.drag.set(0.);
        self.state.set_mode(RefreshIndicatorStatus::Drag);
    }
}

struct RefreshState {
    mode: Cell<Option<RefreshIndicatorStatus>>,
    drag: Cell<f32>,
    value: Cell<f32>,
    revision: Revision,
}

impl RefreshState {
    fn set_mode(&self, mode: RefreshIndicatorStatus) {
        self.mode.set(Some(mode));
        self.revision.bump();
    }
}

/// Flutter's `RefreshIndicator`.
#[derive(Clone)]
pub struct RefreshIndicator {
    child: Widget,
    on_refresh: Rc<dyn Fn(RefreshCompleter)>,
    displacement: f32,
    edge_offset: f32,
    color: Option<Color>,
    background_color: Option<Color>,
    trigger_mode: RefreshIndicatorTriggerMode,
    on_status_change: Option<Rc<dyn Fn(RefreshIndicatorStatus)>>,
    state: Rc<RefreshState>,
}

impl RefreshIndicator {
    #[must_use]
    pub fn new(child: impl Into<Widget>, on_refresh: impl Fn(RefreshCompleter) + 'static) -> Self {
        Self {
            child: child.into(),
            on_refresh: Rc::new(on_refresh),
            displacement: 40.,
            edge_offset: 0.,
            color: None,
            background_color: None,
            trigger_mode: RefreshIndicatorTriggerMode::OnEdge,
            on_status_change: None,
            state: Rc::new(RefreshState {
                mode: Cell::new(None),
                drag: Cell::new(0.),
                value: Cell::new(0.),
                revision: Revision::default(),
            }),
        }
    }

    #[must_use]
    pub fn displacement(mut self, displacement: f32) -> Self {
        self.displacement = displacement;
        self
    }

    #[must_use]
    pub fn edge_offset(mut self, offset: f32) -> Self {
        self.edge_offset = offset;
        self
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[must_use]
    pub fn trigger_mode(mut self, mode: RefreshIndicatorTriggerMode) -> Self {
        self.trigger_mode = mode;
        self
    }

    #[must_use]
    pub fn on_status_change(mut self, callback: impl Fn(RefreshIndicatorStatus) + 'static) -> Self {
        self.on_status_change = Some(Rc::new(callback));
        self
    }

    /// Flutter's `RefreshIndicatorState.show`: refreshes as if pulled.
    pub fn show(&self) {
        self.start_refresh();
    }

    /// The current status, `None` while idle.
    #[must_use]
    pub fn status(&self) -> Option<RefreshIndicatorStatus> {
        self.state.mode.get()
    }

    fn notify(&self, status: RefreshIndicatorStatus) {
        self.state.set_mode(status);
        if let Some(callback) = &self.on_status_change {
            callback(status);
        }
    }

    fn start_refresh(&self) {
        self.state.value.set(1.);
        self.notify(RefreshIndicatorStatus::Snap);
        self.notify(RefreshIndicatorStatus::Refresh);
        (self.on_refresh)(RefreshCompleter {
            state: self.state.clone(),
        });
    }

    fn dismiss(&self, status: RefreshIndicatorStatus) {
        self.state.drag.set(0.);
        self.state.value.set(0.);
        self.notify(status);
        self.state.mode.set(None);
        self.state.revision.bump();
    }

    /// `_handleScrollNotification`.
    fn handle(&self, notification: &ScrollNotification) -> bool {
        let metrics = notification.metrics;
        let at_edge = metrics.pixels <= metrics.min_scroll_extent;
        let mode = self.state.mode.get();
        let refreshing = matches!(
            mode,
            Some(RefreshIndicatorStatus::Refresh | RefreshIndicatorStatus::Snap)
        );
        if refreshing {
            return false;
        }
        match notification.kind {
            ScrollNotificationType::Start => {
                let start = match self.trigger_mode {
                    RefreshIndicatorTriggerMode::OnEdge => at_edge,
                    RefreshIndicatorTriggerMode::Anywhere => true,
                };
                if start && mode.is_none() {
                    self.state.drag.set(0.);
                    self.notify(RefreshIndicatorStatus::Drag);
                }
            }
            ScrollNotificationType::Overscroll if notification.overscroll < 0. => {
                if mode.is_none() && at_edge {
                    self.notify(RefreshIndicatorStatus::Drag);
                }
                if self.state.mode.get().is_some() {
                    self.state
                        .drag
                        .set(self.state.drag.get() - notification.overscroll);
                    self.check_drag(metrics.viewport_dimension);
                }
            }
            ScrollNotificationType::Update if mode.is_some() => {
                if metrics.pixels < metrics.min_scroll_extent {
                    // Bouncing physics: the overscroll is the position.
                    self.state
                        .drag
                        .set(metrics.min_scroll_extent - metrics.pixels);
                } else if notification.delta > 0. {
                    self.state
                        .drag
                        .set((self.state.drag.get() - notification.delta).max(0.));
                }
                self.check_drag(metrics.viewport_dimension);
            }
            ScrollNotificationType::End if mode.is_some() => match self.state.mode.get() {
                Some(RefreshIndicatorStatus::Armed) => self.start_refresh(),
                Some(RefreshIndicatorStatus::Drag) => {
                    self.dismiss(RefreshIndicatorStatus::Canceled)
                }
                _ => {}
            },
            _ => {}
        }
        false
    }

    /// `_checkDragOffset`.
    fn check_drag(&self, extent: f32) {
        let value = (self.state.drag.get() / (extent * DRAG_CONTAINER_EXTENT_PERCENTAGE).max(1.))
            .clamp(0., 1.);
        self.state.value.set(value);
        let mode = self.state.mode.get();
        if mode == Some(RefreshIndicatorStatus::Drag) && value >= 1. {
            self.notify(RefreshIndicatorStatus::Armed);
        } else if mode == Some(RefreshIndicatorStatus::Armed) && value < 1. {
            self.notify(RefreshIndicatorStatus::Drag);
        } else {
            self.state.revision.bump();
        }
    }
}

impl From<RefreshIndicator> for Widget {
    fn from(indicator: RefreshIndicator) -> Self {
        let indicator = Rc::new(indicator);
        let listener: Widget = {
            let indicator = indicator.clone();
            NotificationListener::new(indicator.child.clone())
                .on::<ScrollNotification>(move |notification| indicator.handle(notification))
                .into()
        };
        Widget::stateful_builder(indicator.state.revision.cell(), move |context| {
            let Some(mode) = indicator.state.mode.get() else {
                return Stack::new([listener.clone()]).into();
            };
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let value = indicator.state.value.get();
            // The indicator slides down to `displacement`, scaled by the pull.
            let top = indicator.edge_offset - INDICATOR_SIZE
                + (indicator.displacement + INDICATOR_SIZE) * value.min(1.);
            let spinner = match mode {
                RefreshIndicatorStatus::Refresh | RefreshIndicatorStatus::Snap => {
                    CircularProgressIndicator::new()
                }
                _ => CircularProgressIndicator::new().value(value * 0.75),
            };
            let spinner: Widget = Container::new()
                .width(INDICATOR_SIZE)
                .height(INDICATOR_SIZE)
                .decoration(
                    BoxDecoration::new().shape(BoxShape::Circle).color(
                        indicator
                            .background_color
                            .unwrap_or(scheme.surface_container_high),
                    ),
                )
                .padding(incular_config::EdgeInsets::all(10.))
                .child(SizedBox::new().width(20.).height(20.).child(spinner))
                .into();
            Stack::new([
                listener.clone(),
                Positioned::new(Align::new(Alignment::TOP_CENTER, spinner))
                    .top(top)
                    .left(0.)
                    .right(0.)
                    .into(),
            ])
            .into()
        })
    }
}
