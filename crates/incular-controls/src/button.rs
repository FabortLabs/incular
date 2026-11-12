use crate::ink::InkSplashes;
use crate::styles::{
    ButtonStyle, ButtonVariant, ControlState, IconAlignment, SplashFactory, TapTargetSize,
    mouse_cursor_from_name,
};
use crate::theme::ControlTheme;
use incular_config::{Alignment, Constraints, CrossAxisAlignment, EdgeInsets};
use incular_core::Color;
use incular_core::Offset;
use incular_semantics::{Role as SemanticRole, SemanticActionKind, SemanticState};
use incular_widgets::internal::{
    ActionInkEvent, ActionInteractionController, ActionInteractionState, ActionPolicy,
    ActionSplashPolicy, ActionSurface, DropShadow, ExplicitSemantics,
};

use incular_core::Lerp;
use incular_widgets::{
    Align, Border, BorderRadius, BoxDecoration, ConstrainedBox, Container, Curve, IconTheme,
    MouseCursor, Positioned, Row, SizedBox, Stack, StackFit, Text, Ticker, TweenAnimationBuilder,
    Widget,
};
use smart_default::SmartDefault;
use std::cell::{Cell, RefCell};
use std::{rc::Rc, time::Duration};
use typed_builder::TypedBuilder;

/// Platform-neutral styled push button.
#[derive(Clone, TypedBuilder, SmartDefault)]
pub struct Button {
    #[builder(default, setter(strip_option, into))]
    label: Option<String>,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
    #[builder(default, setter(strip_option, into))]
    icon: Option<Widget>,
    #[builder(default)]
    style: ButtonStyle,
    #[default(true)]
    #[builder(default = true)]
    enabled: bool,
    #[builder(default)]
    focusable_when_disabled: bool,
    #[builder(default)]
    loading: bool,
    #[builder(
        default,
        setter(
            fn transform<F>(callback: F) -> Option<Rc<dyn Fn() + 'static>>
            where
                F: Fn() + 'static,
            {
                Some(Rc::new(callback))
            }
        )
    )]
    on_click: Option<Rc<dyn Fn() + 'static>>,
}

impl Button {
    /// Creates a button with a text label.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self::builder().label(label).build()
    }

    /// Creates a button with a custom child widget.
    #[must_use]
    pub fn with_child(child: impl Into<Widget>) -> Self {
        Self::builder().child(child).build()
    }

    /// Creates a button whose icon and label are composed by the live retained
    /// presentation root, allowing state-aware icon styling and alignment.
    #[must_use]
    pub fn with_icon_label(icon: impl Into<Widget>, label: impl Into<String>) -> Self {
        Self::builder().icon(icon).label(label).build()
    }

    /// Replaces the button's text label and clears any custom child.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.child = None;
        self.icon = None;
        self
    }

    /// Replaces the button's content with an arbitrary widget.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self.label = None;
        self.icon = None;
        self
    }

    /// Content-oriented alias for [`Button::child`].
    #[must_use]
    pub fn content(self, content: impl Into<Widget>) -> Self {
        self.child(content)
    }

    /// Sets the button visual variant.
    #[must_use]
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.style.variant = variant;
        self
    }

    /// Overrides specific button styling properties.
    #[must_use]
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets whether the button is enabled for user interaction.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Keeps a disabled button in the focus order (useful for loading or
    /// explanatory states) while still suppressing activation.
    #[must_use]
    pub fn focusable_when_disabled(mut self, focusable: bool) -> Self {
        self.focusable_when_disabled = focusable;
        self
    }

    /// Shows a busy state while retaining focus and suppressing activation.
    #[must_use]
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Sets click/activation callback.
    #[must_use]
    pub fn on_click(mut self, on_click: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Builds the styled widget tree using the provided or default theme.
    #[must_use]
    pub fn build(&self, theme: &ControlTheme) -> Widget {
        Rc::new(self.clone()).build_shared(Rc::new(theme.clone()))
    }

    /// Builds with the button and theme shared by the retained presentation
    /// builder rather than copied into every button.
    fn build_shared(self: &Rc<Self>, theme: Rc<ControlTheme>) -> Widget {
        let effective_enabled = self.enabled && !self.loading;
        let interaction = ActionInteractionController::new();
        let ink = InkFade::new(interaction.revision());
        let splashes = InkSplashes::new(&interaction.revision());
        let presentation = {
            let button = Rc::clone(self);
            let theme = Rc::clone(&theme);
            let interaction = interaction.clone();
            Widget::stateful_builder(interaction.revision(), move |_| {
                let state = button.control_state(effective_enabled, interaction.state());
                let events = interaction.take_ink_events();
                button.build_presentation(&theme, state, &ink, &splashes, events)
            })
        };

        let mut raw = ActionSurface::with_child(presentation)
            .color(Color::TRANSPARENT)
            .hover_color(Color::TRANSPARENT)
            .pressed_color(Color::TRANSPARENT)
            .focused_color(theme.colors.focus_ring)
            .disabled_color(Color::TRANSPARENT)
            .enabled(effective_enabled)
            .focusable_when_disabled(self.focusable_when_disabled || self.loading)
            .interaction_controller(interaction)
            .policy(ActionPolicy {
                feedback_enabled: self.style.enable_feedback.unwrap_or(true),
                splash: match self.style.splash_factory.unwrap_or_default() {
                    SplashFactory::Ripple => ActionSplashPolicy::Ripple,
                    SplashFactory::Splash => ActionSplashPolicy::Splash,
                    SplashFactory::Sparkle => ActionSplashPolicy::Sparkle,
                    SplashFactory::NoSplash => ActionSplashPolicy::None,
                },
                transition_duration: self.style.animation_duration.unwrap_or_else(|| {
                    Duration::from_millis(u64::from(theme.motion.pressed_duration_ms))
                }),
            })
            .mouse_cursor(
                self.style
                    .mouse_cursor
                    .as_deref()
                    .map(mouse_cursor_from_name)
                    .unwrap_or(if effective_enabled {
                        MouseCursor::Click
                    } else {
                        MouseCursor::Basic
                    }),
            );
        if let Some(cb) = self.on_click.clone()
            && effective_enabled
        {
            raw = raw.on_click(move || cb());
        }
        let semantic_label = self
            .label
            .clone()
            .or_else(|| self.child.as_ref().and_then(Widget::semantic_text))
            .unwrap_or_default();
        let mut semantics = ExplicitSemantics::new(SemanticRole::Button)
            .label(semantic_label)
            .state(SemanticState {
                enabled: effective_enabled,
                focusable: self.enabled || self.focusable_when_disabled || self.loading,
                ..SemanticState::default()
            });
        if effective_enabled {
            semantics =
                semantics.actions([SemanticActionKind::Focus, SemanticActionKind::Activate]);
        } else if self.focusable_when_disabled || self.loading {
            semantics = semantics.actions([SemanticActionKind::Focus]);
        }
        Widget::from(raw).semantics(semantics)
    }

    fn control_state(
        &self,
        effective_enabled: bool,
        interaction: ActionInteractionState,
    ) -> ControlState {
        let mut state = ControlState::from_enabled(effective_enabled);
        if interaction.hovered {
            state.insert(ControlState::HOVERED);
        }
        if interaction.pressed {
            state.insert(ControlState::PRESSED);
        }
        if interaction.focused {
            state.insert(ControlState::FOCUSED);
            state.insert(ControlState::FOCUS_VISIBLE);
        }
        state
    }

    fn build_presentation(
        &self,
        theme: &ControlTheme,
        state: ControlState,
        ink: &InkFade,
        splashes: &InkSplashes,
        events: Vec<ActionInkEvent>,
    ) -> Widget {
        let elevation = self.style.resolve_elevation(state, theme);
        let splash = self.style.splash_factory.unwrap_or_default();
        // Overlays are `InkWell` highlights, which exist only while the button
        // is hovered, focused or pressed.
        let highlighted =
            state.intersects(ControlState::HOVERED | ControlState::FOCUSED | ControlState::PRESSED);
        let overlay = if !highlighted || (state.is_pressed() && splash == SplashFactory::NoSplash) {
            Color::TRANSPARENT
        } else {
            self.style.resolve_overlay_color(state, theme)
        };
        // Flutter's `Material` sets its color at once; the hover, focus and
        // pressed overlays are `InkHighlight`s that fade in and out over it.
        let mut background = self.style.resolve_background(state, theme);
        if self.style.surface_tint_color.is_some() && elevation > 0.0 {
            let tint = self.style.resolve_surface_tint_color(state, theme);
            let alpha = (elevation * 0.04).clamp(0.0, 0.16);
            background = blend_overlay(
                background,
                Color::rgba(
                    tint.red,
                    tint.green,
                    tint.blue,
                    (alpha * 255.0).round() as u8,
                ),
            );
        }
        let fg = self.style.resolve_foreground(state, theme);
        let radius = self
            .style
            .shape
            .as_ref()
            .map(|shape| shape.resolve(state))
            .unwrap_or_else(|| {
                BorderRadius::circular(self.style.border_radius.unwrap_or(theme.button.radius))
            });
        let density = self.style.visual_density.unwrap_or(theme.density);
        let padding = self.style.padding.unwrap_or_else(|| density.padding());
        let fixed_size = self.style.fixed_size;
        let mut height = fixed_size
            .map(|size| size.height)
            .or(self.style.height)
            .unwrap_or_else(|| {
                if self.style.visual_density.is_some() {
                    density.control_height()
                } else {
                    theme.button.height
                }
            });
        if fixed_size.is_none() {
            let minimum_height = self
                .style
                .minimum_size
                .map_or(0.0, |size| size.height.max(0.0));
            height = height.max(minimum_height);
            if let Some(maximum) = self.style.maximum_size {
                height = height.min(maximum.height.max(minimum_height));
            }
        }

        let label: Option<Widget> = self.label.as_ref().map(|label| {
            Widget::from(
                Text::new(label.clone()).style(
                    self.style
                        .text_style
                        .clone()
                        .unwrap_or_else(|| theme.typography.body.clone())
                        .color(fg),
                ),
            )
        });
        let icon: Option<Widget> = self.icon.as_ref().map(|icon| {
            let icon_color = self
                .style
                .icon_color
                .as_ref()
                .map_or(fg, |value| value.resolve(state));
            let icon_size = self
                .style
                .icon_size
                .as_ref()
                .map_or(theme.metrics.icon_size, |value| {
                    value.resolve(state).max(0.0)
                });
            IconTheme::new(icon.clone())
                .color(icon_color)
                .size(icon_size)
                .into()
        });
        let mut content: Widget = match (icon, label) {
            (Some(icon), Some(label)) => {
                let spacing: Widget = SizedBox::new().width(theme.metrics.small_spacing).into();
                let children = match self.style.icon_alignment.unwrap_or_default() {
                    IconAlignment::Start => [icon, spacing, label],
                    IconAlignment::End => [label, spacing, icon],
                };
                Row::new(children)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .into()
            }
            (Some(icon), None) => icon,
            (None, Some(label)) => label,
            (None, None) => self
                .child
                .clone()
                .unwrap_or_else(|| incular_widgets::SizedBox::shrink().into()),
        };

        if let Some(builder) = self.style.foreground_builder.as_ref() {
            content = builder.build(content, state);
        }

        let border = self
            .style
            .side
            .as_ref()
            .map(|side| side.resolve(state))
            .or(self.style.border)
            .unwrap_or_else(|| match self.style.variant {
                ButtonVariant::Primary | ButtonVariant::Danger | ButtonVariant::Destructive => {
                    Border::new(0.0, Color::TRANSPARENT)
                }
                ButtonVariant::Default | ButtonVariant::Standard => {
                    Border::new(theme.button.border_width, theme.colors.border)
                }
                ButtonVariant::Ghost => Border::new(0.0, Color::TRANSPARENT),
            });

        let shrink_wrap = self.style.fixed_size.is_none();
        let content = if shrink_wrap {
            // Shrink-wrap both axes: the minimum height below then centers
            // short content, and tall content grows the button.
            Align::new(self.style.alignment.unwrap_or(Alignment::CENTER), content)
                .width_factor(1.0)
                .height_factor(1.0)
                .into()
        } else {
            content
        };

        // The ink layer: the overlay with Flutter's `InkWell` highlight fade
        // (`getFadeDurationForType`: 200 ms pressed, 50 ms hover and focus),
        // and the border painted over it as `Material` paints its shape.
        // An absent overlay keeps the hover overlay's hue at zero alpha, so
        // the fade changes only alpha, as `InkHighlight` does.
        let resting_overlay = {
            let hue = self
                .style
                .resolve_overlay_color(state | ControlState::HOVERED, theme);
            Color::rgba(hue.red, hue.green, hue.blue, 0)
        };
        let overlay = if overlay.alpha == 0 {
            resting_overlay
        } else {
            overlay
        };
        let fade = if state.is_pressed() {
            Duration::from_millis(200)
        } else {
            Duration::from_millis(50)
        };
        let highlight_color = ink.follow(overlay, fade);
        let mut border_decoration = BoxDecoration::new().border_radius(radius);
        if has_visible_border(&border) {
            border_decoration = border_decoration.border(border);
        }
        // Ink splashes (Flutter `InkWell` with `ButtonStyle.splashFactory`) in
        // the overlay's pressed color, clipped to the shape and painted under
        // the content, as `Material` paints its ink features before its child.
        if !events.is_empty() {
            let color = self
                .style
                .resolve_overlay_color(state | ControlState::PRESSED, theme);
            splashes.handle(events, splash, color);
        }
        let mut content_layer = Container::new()
            .padding(padding)
            .decoration(border_decoration)
            .child(content);
        if !shrink_wrap {
            content_layer =
                content_layer.alignment(self.style.alignment.unwrap_or(Alignment::CENTER));
        }
        // The ink features cover the whole button (the `InkWell` reference
        // box): the highlight, then the splashes, then the content and the
        // shape's border. `Passthrough` gives the content the constraints the
        // ink layer gets, so the ink never changes the button's layout. The
        // highlight is the splash layer's background. A button that never
        // showed ink has no ink child: the keyed content keeps its element
        // when the ink child is inserted before it, and buttons mounted while
        // scrolling a long list stay as cheap as they were without ink.
        let content_layer = Widget::from(content_layer).with_key(CONTENT_KEY);
        let ink_layer = if splashes.needs_layer(highlight_color.alpha != 0) {
            let mut highlight = Container::new().decoration(
                BoxDecoration::new()
                    .color(highlight_color)
                    .border_radius(radius),
            );
            highlight = highlight.child(
                splashes
                    .layer(radius)
                    .unwrap_or_else(|| SizedBox::shrink().into()),
            );
            Stack::new([Positioned::fill(highlight).into(), content_layer])
        } else {
            Stack::new([content_layer])
        }
        .fit(StackFit::Passthrough);
        let decoration = BoxDecoration::new().color(background).border_radius(radius);
        // The theme's control height is a minimum, as Flutter's
        // `ButtonStyle.minimumSize` is: multi-line or tall content grows the
        // button. Only an explicit `fixed_size` or `height` pins it.
        let exact_height = fixed_size.is_some() || self.style.height.is_some();
        let mut decorated = Container::new().decoration(decoration).child(ink_layer);
        if exact_height {
            decorated = decorated.height(height);
        }
        if let Some(size) = fixed_size {
            decorated = decorated.width(size.width);
        }

        let mut min_width = self
            .style
            .minimum_size
            .map_or(0.0, |size| size.width.max(0.0));
        let mut min_height = self
            .style
            .minimum_size
            .map_or(0.0, |size| size.height.max(0.0));
        let mut max_width = self
            .style
            .maximum_size
            .map_or(f32::INFINITY, |size| size.width.max(min_width));
        let mut max_height = self
            .style
            .maximum_size
            .map_or(f32::INFINITY, |size| size.height.max(min_height));
        if !exact_height {
            min_height = min_height.max(height);
            max_height = max_height.max(min_height);
        }
        if let Some(size) = fixed_size {
            min_width = size.width.max(0.0);
            max_width = min_width;
            min_height = size.height.max(0.0);
            max_height = min_height;
        }
        if min_width > 0.0 || min_height > 0.0 || max_width.is_finite() || max_height.is_finite() {
            decorated = decorated.constraints(Constraints::new(
                min_width, max_width, min_height, max_height,
            ));
        }

        let mut decorated: Widget = decorated.into();
        if let Some(builder) = self.style.background_builder.as_ref() {
            decorated = builder.build(decorated, state);
        }

        // Flutter's `_MaterialInterior` tweens elevation and shadow color over
        // `animationDuration` (`kThemeChangeDuration`) with `fastOutSlowIn`.
        // A button that can rise keeps its shadow layer at every elevation,
        // so the content keeps its state while the shadow animates.
        let can_rise = [
            ControlState::empty(),
            ControlState::HOVERED,
            ControlState::FOCUSED,
            ControlState::PRESSED,
            ControlState::DISABLED,
            ControlState::SELECTED,
        ]
        .into_iter()
        .any(|probe| self.style.resolve_elevation(probe | state, theme) > 0.0);
        let visual = if can_rise {
            let shadow = self.style.resolve_shadow_color(state, theme);
            TweenAnimationBuilder::new(
                (elevation, shadow),
                self.style
                    .animation_duration
                    .unwrap_or(Duration::from_millis(200)),
                |_, (elevation, shadow): (f32, Color), child| -> Widget {
                    let child = child.unwrap_or_else(|| SizedBox::shrink().into());
                    let shadow = if elevation > 0.0 {
                        shadow
                    } else {
                        Color::TRANSPARENT
                    };
                    DropShadow::new(
                        Offset::new(0.0, elevation * 0.18),
                        (elevation * 0.55).max(1.0),
                        shadow,
                        child,
                    )
                    .into()
                },
            )
            .curve(Curve::FastOutSlowIn)
            .child(decorated)
            .into()
        } else {
            decorated
        };
        // Flutter's `_InputPadding` for `MaterialTapTargetSize.padded`: the
        // target is at least 48x48 with the visual centered in it, which
        // keeps its own size.
        if self.style.tap_target_size == Some(TapTargetSize::Padded) && fixed_size.is_none() {
            ConstrainedBox::new(
                Constraints::new(48.0, f32::INFINITY, 48.0, f32::INFINITY),
                Align::new(Alignment::CENTER, visual)
                    .width_factor(1.0)
                    .height_factor(1.0),
            )
            .into()
        } else {
            visual
        }
    }
}

fn blend_overlay(base: Color, overlay: Color) -> Color {
    let source_alpha = overlay.alpha as f32 / 255.0;
    if source_alpha <= 0.0 {
        return base;
    }
    let destination_alpha = base.alpha as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if output_alpha <= f32::EPSILON {
        return Color::TRANSPARENT;
    }
    let channel = |source: u8, destination: u8| {
        ((source as f32 * source_alpha
            + destination as f32 * destination_alpha * (1.0 - source_alpha))
            / output_alpha)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(
        channel(overlay.red, base.red),
        channel(overlay.green, base.green),
        channel(overlay.blue, base.blue),
        (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn has_visible_border(border: &Border) -> bool {
    [border.top, border.right, border.bottom, border.left]
        .into_iter()
        .any(|side| side.width > 0.0 && side.color.alpha > 0)
}

impl From<Button> for Widget {
    fn from(value: Button) -> Self {
        let value = Rc::new(value);
        let semantic_label = value
            .label
            .clone()
            .or_else(|| value.child.as_ref().and_then(Widget::semantic_text));
        let widget = Widget::from(incular_widgets::Builder::new(move |context| {
            value.build_shared(crate::theme::current_control_theme_shared(context))
        }));
        semantic_label.map_or(widget.clone(), |label| widget.accessibility_label(label))
    }
}

/// Declares a preset button: a [`Button`] with a different default style.
macro_rules! preset_button {
    ($(#[$meta:meta])* $name:ident, $style:expr) => {
        $(#[$meta])*
        #[derive(Clone, TypedBuilder)]
        pub struct $name {
            #[builder(default, setter(strip_option, into))]
            label: Option<String>,
            #[builder(default, setter(strip_option, into))]
            child: Option<Widget>,
            #[builder(default = $style)]
            style: ButtonStyle,
            #[builder(default = true)]
            enabled: bool,
            #[builder(default)]
            focusable_when_disabled: bool,
            #[builder(default)]
            loading: bool,
            #[builder(
                default,
                setter(
                    fn transform<F>(callback: F) -> Option<Rc<dyn Fn() + 'static>>
                    where
                        F: Fn() + 'static,
                    {
                        Some(Rc::new(callback))
                    }
                )
            )]
            on_click: Option<Rc<dyn Fn() + 'static>>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::builder().build()
            }
        }

        impl $name {
            #[must_use]
            pub fn new(label: impl Into<String>) -> Self {
                Self::builder().label(label).build()
            }

            #[must_use]
            pub fn with_child(child: impl Into<Widget>) -> Self {
                Self::builder().child(child).build()
            }

            #[must_use]
            pub fn label(mut self, label: impl Into<String>) -> Self {
                self.label = Some(label.into());
                self.child = None;
                self
            }

            #[must_use]
            pub fn child(mut self, child: impl Into<Widget>) -> Self {
                self.child = Some(child.into());
                self.label = None;
                self
            }

            #[must_use]
            pub fn content(self, content: impl Into<Widget>) -> Self {
                self.child(content)
            }

            #[must_use]
            pub fn style(mut self, style: ButtonStyle) -> Self {
                self.style = style;
                self
            }
            #[must_use]
            pub fn enabled(mut self, enabled: bool) -> Self {
                self.enabled = enabled;
                self
            }
            #[must_use]
            pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
                self.on_click = Some(Rc::new(callback));
                self
            }
            #[must_use]
            pub fn focusable_when_disabled(mut self, value: bool) -> Self {
                self.focusable_when_disabled = value;
                self
            }
            #[must_use]
            pub fn loading(mut self, value: bool) -> Self {
                self.loading = value;
                self
            }

            fn into_button(self) -> Button {
                Button {
                    label: self.label,
                    child: self.child,
                    icon: None,
                    style: self.style,
                    enabled: self.enabled,
                    focusable_when_disabled: self.focusable_when_disabled,
                    loading: self.loading,
                    on_click: self.on_click,
                }
            }
        }

        impl From<$name> for Widget {
            fn from(value: $name) -> Self {
                value.into_button().into()
            }
        }
    };
}

preset_button!(
    /// Primary high-emphasis action button.
    PrimaryButton,
    ButtonStyle::new().variant(ButtonVariant::Primary)
);

preset_button!(
    /// Ghost/flat button for toolbars and lightweight actions.
    GhostButton,
    ButtonStyle::new().variant(ButtonVariant::Ghost)
);

preset_button!(
    /// Compact square icon button.
    IconButton,
    ButtonStyle::new()
        .variant(ButtonVariant::Ghost)
        .padding(EdgeInsets::all(4.0))
        .height(28.0)
);

/// The fade of a button's ink overlay: Flutter's `InkHighlight` alpha
/// animation. It runs on a frame-scheduler [`Ticker`] only while a fade is in
/// progress and rebuilds the presentation through the interaction revision,
/// so a resting button adds nothing to a frame.
/// The key of a button's content in its ink stack.
const CONTENT_KEY: u64 = 0;

struct InkFade {
    /// The color the last build asked for.
    target: Cell<Option<Color>>,
    fade: Rc<FadeState>,
    ticker: Rc<RefCell<Option<Ticker>>>,
}

struct FadeState {
    from: Cell<Color>,
    to: Cell<Color>,
    current: Cell<Color>,
    duration: Cell<Duration>,
    revision: Rc<Cell<u64>>,
}

impl InkFade {
    fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            target: Cell::new(None),
            fade: Rc::new(FadeState {
                from: Cell::new(Color::TRANSPARENT),
                to: Cell::new(Color::TRANSPARENT),
                current: Cell::new(Color::TRANSPARENT),
                duration: Cell::new(Duration::ZERO),
                revision,
            }),
            ticker: Rc::new(RefCell::new(None)),
        }
    }

    /// The overlay to paint this build while fading towards `target` over
    /// `duration`. The first build shows the target at once.
    fn follow(&self, target: Color, duration: Duration) -> Color {
        let fade = &self.fade;
        match self.target.replace(Some(target)) {
            None => fade.current.set(target),
            Some(previous) if previous != target => {
                fade.from.set(fade.current.get());
                fade.to.set(target);
                fade.duration.set(duration);
                if duration.is_zero() || incular_widgets::disable_animations() {
                    fade.current.set(target);
                    self.stop();
                } else {
                    self.start();
                }
            }
            Some(_) => {}
        }
        fade.current.get()
    }

    /// Starts (or restarts) the fade from the next frame.
    fn start(&self) {
        let mut slot = self.ticker.borrow_mut();
        let ticker = slot.get_or_insert_with(|| {
            let fade = Rc::downgrade(&self.fade);
            let own = Rc::downgrade(&self.ticker);
            Ticker::new(move |elapsed| {
                let Some(fade) = fade.upgrade() else {
                    return;
                };
                let span = fade.duration.get().as_secs_f32();
                let t = if span > 0.0 {
                    (elapsed.as_secs_f32() / span).min(1.0)
                } else {
                    1.0
                };
                fade.current.set(fade.from.get().lerp(&fade.to.get(), t));
                fade.revision.set(fade.revision.get().wrapping_add(1));
                if t >= 1.0
                    && let Some(own) = own.upgrade()
                    && let Ok(slot) = own.try_borrow()
                    && let Some(ticker) = slot.as_ref()
                {
                    ticker.stop();
                }
            })
        });
        ticker.stop();
        ticker.start();
    }

    fn stop(&self) {
        if let Some(ticker) = self.ticker.borrow().as_ref() {
            ticker.stop();
        }
    }
}
