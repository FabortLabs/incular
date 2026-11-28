//! Design-system-neutral controls for a widget preview host.

use incular_config::MainAxisSize;
use incular_core::Transform;
use incular_widgets::{Builder, RawTooltip, Row, SizedBox, Text, TransformationController, Widget};

use crate::{Button, ButtonVariant, slider};

/// The controls and viewport share this transform. External edits refresh
/// the percentage and slider, and zoom controls replace the transform with
/// a uniform scale (or the identity on reset).
#[derive(Clone, Debug)]
pub struct PreviewZoomControls {
    controller: TransformationController,
}

impl PreviewZoomControls {
    pub const MIN_SCALE: f32 = 1.;
    pub const MAX_SCALE: f32 = 4.;
    pub const ZOOM_STEP: f32 = 0.25;

    #[must_use]
    pub const fn new(controller: TransformationController) -> Self {
        Self { controller }
    }

    #[must_use]
    pub fn scale(&self) -> f32 {
        let scale = self.controller.value().coefficients()[0] as f32;
        if scale.is_finite() {
            scale
        } else {
            Self::MIN_SCALE
        }
    }

    pub fn set_scale(&self, scale: f32) {
        if scale.is_finite() {
            self.controller.set_value(Transform::scale(
                scale.clamp(Self::MIN_SCALE, Self::MAX_SCALE),
            ));
        }
    }

    pub fn zoom_in(&self) {
        let scale = self.scale();
        if scale < Self::MAX_SCALE {
            self.set_scale(scale + Self::ZOOM_STEP);
        }
    }

    pub fn zoom_out(&self) {
        let scale = self.scale();
        if scale > Self::MIN_SCALE {
            self.set_scale(scale - Self::ZOOM_STEP);
        }
    }

    pub fn reset(&self) {
        self.controller.set_value(Transform::IDENTITY);
    }
}

fn button(
    label: &'static str,
    key: &'static str,
    enabled: bool,
    click: impl Fn() + 'static,
) -> Widget {
    let button = Button::new(label)
        .variant(ButtonVariant::Ghost)
        .enabled(enabled)
        .on_click(click);
    Widget::from(RawTooltip::new(label, button)).with_key(key)
}

impl From<PreviewZoomControls> for Widget {
    fn from(controls: PreviewZoomControls) -> Self {
        // Reuse the slider's value and focus node when the shared preview
        // transform changes, rather than replacing a focused control.
        let slider = slider::Root::new()
            .range(
                PreviewZoomControls::MIN_SCALE,
                PreviewZoomControls::MAX_SCALE,
            )
            .step(f32::EPSILON)
            .interaction_step(if cfg!(any(target_os = "ios", target_os = "macos")) {
                0.3
            } else {
                0.15
            })
            .track_extent(100.)
            .on_value_change({
                let controls = controls.clone();
                move |scale| controls.set_scale(scale)
            });
        Builder::new(move |_| {
            let scale = controls.scale();
            let label = format!("{:.0}%", (f64::from(scale) * 100.).round());
            let out = controls.clone();
            let input = controls.clone();
            let reset = controls.clone();
            Row::new([
                button(
                    "Zoom out",
                    "preview.zoom.out",
                    scale > PreviewZoomControls::MIN_SCALE,
                    move || out.zoom_out(),
                ),
                Widget::from(slider.clone().value(scale)).with_key("preview.zoom.slider"),
                button(
                    "Zoom in",
                    "preview.zoom.in",
                    scale < PreviewZoomControls::MAX_SCALE,
                    move || input.zoom_in(),
                ),
                button(
                    "Reset zoom",
                    "preview.zoom.reset",
                    scale != PreviewZoomControls::MIN_SCALE,
                    move || reset.reset(),
                ),
                Widget::from(SizedBox::new().width(42.).child(Text::new(&label)))
                    .with_key("preview.zoom.percentage"),
            ])
            .main_axis_size(MainAxisSize::Min)
            .into()
        })
        .into()
    }
}

/// Requests a full previewer restart through the host-provided callback.
///
/// This is separate from restarting an individual mounted preview and does
/// not itself own or launch a preview process.
#[must_use]
pub fn previewer_restart_button(request_hot_restart: impl Fn() + 'static) -> Widget {
    button(
        "Restart the Widget Previewer",
        "preview.restart.host",
        true,
        request_hot_restart,
    )
}

/// A selected-file filter toggle. Without an Editor service it occupies no
/// space; a failed preference write leaves the setting and previews intact.
pub fn preview_file_filter_toggle<T: 'static, E: 'static>(
    filter: incular_widgets::widget_previews::file_filter::PreviewFileFilter<T>,
    persist: impl Fn(&'static str, bool) -> Result<(), E> + 'static,
    on_error: impl Fn(E) + 'static,
) -> Widget {
    let persist = std::rc::Rc::new(persist);
    let on_error = std::rc::Rc::new(on_error);
    Builder::new(move |_| {
        if !filter.toggle_visible() {
            return SizedBox::shrink().into();
        }
        let enabled = filter.enabled();
        let filter = filter.clone();
        let persist = persist.clone();
        let on_error = on_error.clone();
        let toggle = Button::new("Filter by selected file")
            .variant(if enabled {
                ButtonVariant::Default
            } else {
                ButtonVariant::Ghost
            })
            .on_click(move || {
                if let Err(error) = filter.toggle(|key, value| persist(key, value)) {
                    on_error(error);
                }
            });
        Widget::from(RawTooltip::new("Filter by selected file", toggle))
            .with_key("preview.filter.selected-file")
    })
    .into()
}

/// Borderless preview search editing and its conditional clear action.
#[derive(Clone)]
pub struct PreviewSearchControls {
    search: incular_widgets::widget_previews::search::PreviewSearch,
    focus_node: incular_widgets::FocusNode,
}

impl PreviewSearchControls {
    #[must_use]
    pub fn new(search: incular_widgets::widget_previews::search::PreviewSearch) -> Self {
        Self {
            search,
            focus_node: incular_widgets::FocusNode::new(),
        }
    }

    #[must_use]
    pub fn focus_node(mut self, node: incular_widgets::FocusNode) -> Self {
        self.focus_node = node;
        self
    }
}

impl From<PreviewSearchControls> for Widget {
    fn from(controls: PreviewSearchControls) -> Self {
        use incular_widgets::{
            EditableText, Expanded, GestureDetector, HitTestBehavior, Semantics, TextFieldTapRegion,
        };
        let input = Builder::new({
            let editor = controls.search.editing_controller();
            let focus = controls.focus_node;
            move |context| {
                let theme = crate::theme::current_control_theme(context);
                Widget::from(
                    EditableText::new(editor.clone())
                        .placeholder("Search previews")
                        .placeholder_color(theme.colors.foreground_muted)
                        .size(incular_core::Size::new(0., 26.))
                        .content_padding(incular_config::EdgeInsets::symmetric(8., 4.))
                        .style(theme.typography.body.clone().color(theme.colors.foreground))
                        .cursor_color(theme.colors.foreground)
                        .focus_node(focus.clone()),
                )
                .with_key("preview.search.editor")
            }
        });
        let clear = Builder::new(move |context| {
            if controls.search.query().is_empty() {
                return SizedBox::shrink().into();
            }
            let theme = crate::theme::current_control_theme(context);
            let clear_on_tap = controls.search.clone();
            let clear_on_action = controls.search.clone();
            let hit = GestureDetector::new(SizedBox::new().width(40.).height(26.).child(
                incular_widgets::Center::new(Text::new("×").color(theme.colors.foreground)),
            ))
            .behavior(HitTestBehavior::Opaque)
            .on_tap(move || clear_on_tap.clear());
            let semantic = Semantics::new(hit)
                .role(incular_semantics::Role::Button)
                .label("Clear search")
                .on_tap(move || clear_on_action.clear());
            Widget::from(TextFieldTapRegion::new(RawTooltip::new(
                "Clear search",
                semantic,
            )))
            .with_key("preview.search.clear")
        });
        Row::new([Widget::from(Expanded::new(input)), clear.into()]).into()
    }
}

/// Restarts just the mounted preview, not the host or any other preview.
pub fn preview_soft_restart_button(
    restart: incular_widgets::widget_previews::restart::PreviewSoftRestart,
) -> Widget {
    button("Hot restart", "preview.restart.soft", true, move || {
        restart.request();
    })
}
