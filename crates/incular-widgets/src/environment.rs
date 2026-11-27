//! Ambient configuration and inherited environment widgets.

use incular_config::{
    Locale, LocaleResolver, LocalizationCatalog, LocalizedMessage, RuntimeEnvironment,
    TextDirection,
};
use incular_core::Color;
use incular_scroll::{ScrollController, ScrollPhysics};
use incular_text::{TextAlign, TextHeightBehavior, TextOverflow, TextScaler, TextStyle};
use std::{any::Any, rc::Rc};
use typed_builder::TypedBuilder;

use crate::{BuildContext, Builder, LayoutBuilder, Widget};

pub use incular_config::{ContentSensitivity, MediaQueryEdges};

/// A typed, window-local environment snapshot for descendants that need
/// viewport, scale, safe-area, brightness, or accessibility information.
/// The runtime installs its authoritative snapshot on each `WidgetTree`; this
/// descriptor is useful for isolated subtrees and deterministic tests.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct MediaQuery {
    #[builder(setter(transform = |data: RuntimeEnvironment| normalize_media_query(data)))]
    data: RuntimeEnvironment,
    #[builder(setter(into))]
    child: Widget,
}

/// The renderer-neutral data carried by [`MediaQuery`]. Incular's runtime
/// environment already owns viewport, scale, safe-area, brightness, and
/// accessibility values, so this is an alias rather than a second snapshot
/// type to keep synchronized.
pub type MediaQueryData = RuntimeEnvironment;

impl MediaQuery {
    #[must_use]
    pub fn new(data: RuntimeEnvironment, child: impl Into<Widget>) -> Self {
        Self {
            data: normalize_media_query(data),
            child: child.into(),
        }
    }

    /// Projects the selected view's metrics and installs a live MediaQuery.
    /// A surrounding MediaQuery supplies platform preferences, but never
    /// the selected view's dimensions, insets or gesture geometry.
    #[must_use]
    pub fn from_view(view: crate::ViewController, child: impl Into<Widget>) -> Widget {
        let child = child.into();
        Widget::stateful_builder(view.revision_cell(), move |context| {
            // An absent MediaQuery is distinct from the root's retained
            // runtime snapshot: use this view's own platform values then.
            let parent = context.depend_on_shared::<RuntimeEnvironment>();
            let data = Self::data_from_view(&view, parent.as_deref());
            Self::new(data, child.clone()).into()
        })
    }

    /// Installs the media scope for an application hosted by the retained
    /// runtime's current view. A native View scope can already supply it.
    pub(crate) fn from_runtime_view(child: Widget) -> Widget {
        Builder::new(move |context| {
            Self::new(context.effective_media_query(), child.clone()).into()
        })
        .into()
    }

    /// Snapshots the metrics of one view using an optional explicit source
    /// for platform preferences (Flutter's `MediaQueryData.fromView`).
    #[must_use]
    pub fn data_from_view(
        view: &crate::ViewController,
        platform: Option<&MediaQueryData>,
    ) -> MediaQueryData {
        let mut data = view.data().environment;
        data.navigation_mode = incular_config::NavigationMode::Traditional;
        if let Some(platform) = platform {
            data.text_scale = platform.text_scale;
            data.brightness = platform.brightness;
            data.accessible_navigation = platform.accessible_navigation;
            data.invert_colors = platform.invert_colors;
            data.reduced_motion = platform.reduced_motion;
            data.bold_text = platform.bold_text;
            data.high_contrast = platform.high_contrast;
            data.on_off_switch_labels = platform.on_off_switch_labels;
            data.supports_announce = platform.supports_announce;
            data.always_use_24_hour_format = platform.always_use_24_hour_format;
            data.navigation_mode = platform.navigation_mode;
            data.supports_showing_system_context_menu =
                platform.supports_showing_system_context_menu;
            data.line_height_scale_factor_override = platform
                .line_height_scale_factor_override
                .or(data.line_height_scale_factor_override);
            data.letter_spacing_override =
                platform.letter_spacing_override.or(data.letter_spacing_override);
            data.word_spacing_override =
                platform.word_spacing_override.or(data.word_spacing_override);
            data.paragraph_spacing_override =
                platform.paragraph_spacing_override.or(data.paragraph_spacing_override);
        }
        data
    }

    /// Returns the platform text scaling policy captured from this view.
    /// A previously captured policy is unaffected by later view updates.
    #[must_use]
    pub fn system_text_scaler_from_view(view: &crate::ViewController) -> TextScaler {
        TextScaler::system(Self::data_from_view(view, None).text_scale)
    }

    /// Subscribes to all data in the nearest MediaQuery, if present.
    #[must_use]
    pub fn maybe_of(context: &BuildContext<'_>) -> Option<MediaQueryData> {
        context.depend_on()
    }

    /// Subscribes to all data in the nearest MediaQuery.
    ///
    /// Panics if the context has no MediaQuery ancestor.
    #[must_use]
    #[track_caller]
    pub fn of(context: &BuildContext<'_>) -> MediaQueryData {
        Self::maybe_of(context).expect("No MediaQuery widget ancestor found")
    }

    /// Reads and subscribes to the nearest media size only.
    #[must_use]
    pub fn maybe_size_of(context: &BuildContext<'_>) -> Option<incular_core::Size> {
        context.media_query_size()
    }

    #[must_use]
    #[track_caller]
    pub fn size_of(context: &BuildContext<'_>) -> incular_core::Size {
        Self::maybe_size_of(context).expect("No MediaQuery widget ancestor found")
    }

    /// Reads and subscribes to the nearest media width only.
    #[must_use]
    pub fn maybe_width_of(context: &BuildContext<'_>) -> Option<f32> {
        context.media_query_width()
    }

    #[must_use]
    #[track_caller]
    pub fn width_of(context: &BuildContext<'_>) -> f32 {
        Self::maybe_width_of(context).expect("No MediaQuery widget ancestor found")
    }

    /// Reads and subscribes to the nearest media height only.
    #[must_use]
    pub fn maybe_height_of(context: &BuildContext<'_>) -> Option<f32> {
        context.media_query_height()
    }

    #[must_use]
    #[track_caller]
    pub fn height_of(context: &BuildContext<'_>) -> f32 {
        Self::maybe_height_of(context).expect("No MediaQuery widget ancestor found")
    }

    /// Reads the nearest media orientation, only subscribing when it changes.
    #[must_use]
    pub fn maybe_orientation_of(context: &BuildContext<'_>) -> Option<Orientation> {
        context.media_query_orientation()
    }

    #[must_use]
    #[track_caller]
    pub fn orientation_of(context: &BuildContext<'_>) -> Orientation {
        Self::maybe_orientation_of(context).expect("No MediaQuery widget ancestor found")
    }

    /// Reads only the closest MediaQuery's animation preference.
    #[must_use]
    pub fn maybe_disable_animations_of(context: &BuildContext<'_>) -> Option<bool> {
        context.media_query_disable_animations()
    }

    #[must_use]
    #[track_caller]
    pub fn disable_animations_of(context: &BuildContext<'_>) -> bool {
        Self::maybe_disable_animations_of(context).expect("No MediaQuery widget ancestor found")
    }

    /// Removes selected safe padding from the nearest media scope.
    #[must_use]
    pub fn remove_padding(child: impl Into<Widget>, edges: MediaQueryEdges) -> Widget {
        Self::map_query(child, move |data| data.remove_padding(edges))
    }

    /// Removes selected transient insets from the nearest media scope.
    #[must_use]
    pub fn remove_view_insets(child: impl Into<Widget>, edges: MediaQueryEdges) -> Widget {
        Self::map_query(child, move |data| data.remove_view_insets(edges))
    }

    /// Removes selected persistent view padding from the nearest media scope.
    #[must_use]
    pub fn remove_view_padding(child: impl Into<Widget>, edges: MediaQueryEdges) -> Widget {
        Self::map_query(child, move |data| data.remove_view_padding(edges))
    }

    fn map_query(
        child: impl Into<Widget>,
        map: impl Fn(RuntimeEnvironment) -> RuntimeEnvironment + 'static,
    ) -> Widget {
        let child = child.into();
        Builder::new(move |context| {
            Self::new(map(context.effective_media_query()), child.clone()).into()
        })
        .into()
    }

    /// Creates a local copy of the nearest media data with all four text
    /// style overrides replaced. Passing None clears that inherited override.
    #[must_use]
    pub fn apply_text_style_overrides(
        child: impl Into<Widget>,
        line_height_scale_factor: Option<f32>,
        letter_spacing: Option<f32>,
        word_spacing: Option<f32>,
        paragraph_spacing: Option<f32>,
    ) -> Widget {
        Self::map_query(child, move |mut data| {
            data.line_height_scale_factor_override = line_height_scale_factor;
            data.letter_spacing_override = letter_spacing;
            data.word_spacing_override = word_spacing;
            data.paragraph_spacing_override = paragraph_spacing;
            data
        })
    }

    #[must_use]
    pub fn data(&self) -> &RuntimeEnvironment {
        &self.data
    }

    #[must_use]
    pub fn into_data(self) -> RuntimeEnvironment {
        self.data
    }
}

fn normalize_media_query(data: RuntimeEnvironment) -> RuntimeEnvironment {
    let explicitly_zero = data.text_scale == 0.;
    let mut data = data.normalized();
    if explicitly_zero {
        // Unlike a native configuration, a scoped linear scaler may
        // intentionally make text invisible with a factor of zero.
        data.text_scale = 0.;
    }
    data
}

impl From<MediaQuery> for Widget {
    fn from(value: MediaQuery) -> Self {
        Widget::environment_scope(value.data, value.child)
    }
}

/// Typed localization scope for a retained subtree.
///
/// Catalog storage remains application-owned through [`LocalizationCatalog`]
/// and is exposed to deferred builders as `(Locale, Rc<dyn
/// LocalizationCatalog>)`. This keeps locale selection in `incular-config`
/// without recreating Flutter's delegate/inherited-widget hierarchy.
#[derive(Clone, TypedBuilder)]
pub struct Localizations {
    #[builder(setter(into))]
    locale: Locale,
    #[builder(setter(
        fn transform<C>(catalog: C) -> Rc<dyn LocalizationCatalog>
        where
            C: LocalizationCatalog + 'static,
        {
            Rc::new(catalog)
        }
    ))]
    catalog: Rc<dyn LocalizationCatalog>,
    #[builder(default)]
    application_level: bool,
    #[builder(setter(into))]
    child: Widget,
}

impl Localizations {
    #[must_use]
    pub fn new(
        locale: Locale,
        catalog: impl LocalizationCatalog + 'static,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            locale,
            catalog: Rc::new(catalog),
            application_level: false,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Reads and subscribes to the nearest localization scope.
    #[must_use]
    pub fn maybe_locale_of(context: &BuildContext<'_>) -> Option<Locale> {
        context
            .depend_on::<(Locale, Rc<dyn LocalizationCatalog>)>()
            .map(|(locale, _)| locale)
    }

    /// Reads and subscribes to the nearest localization scope.
    ///
    /// Panics when the context has no Localizations ancestor.
    #[must_use]
    #[track_caller]
    pub fn locale_of(context: &BuildContext<'_>) -> Locale {
        Self::maybe_locale_of(context)
            .expect("Localizations::locale_of requires a Localizations ancestor")
    }

    pub(crate) fn with_shared_catalog(
        locale: Locale,
        catalog: Rc<dyn LocalizationCatalog>,
        child: Widget,
    ) -> Self {
        Self {
            locale,
            catalog,
            application_level: false,
            child,
        }
    }

    /// Leaves semantic pronunciation to the application instead of marking
    /// this locale as an override on every descendant accessibility node.
    #[must_use]
    pub fn application_level(mut self, value: bool) -> Self {
        self.application_level = value;
        self
    }

    #[must_use]
    pub fn catalog(&self) -> &Rc<dyn LocalizationCatalog> {
        &self.catalog
    }

    /// Replaces the preferred locale while retaining the same catalog and
    /// child subtree. Rebuilding a scope with this value invalidates only
    /// descendants that read the localization environment.
    #[must_use]
    pub fn with_locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    /// Resolves a message key through ICU4X locale fallback and this scope's
    /// application-owned catalog.
    #[must_use]
    pub fn resolve(&self, key: &str) -> Option<LocalizedMessage<'_>> {
        LocaleResolver::resolve_message(
            std::slice::from_ref(&self.locale),
            self.catalog.as_ref(),
            key,
        )
    }

    #[must_use]
    pub fn text(&self, key: &str) -> Option<String> {
        self.resolve(key).map(|message| message.value.to_owned())
    }

    #[must_use]
    pub fn text_direction(&self) -> TextDirection {
        LocaleResolver::text_direction(&self.locale)
    }
}

impl From<Localizations> for Widget {
    fn from(value: Localizations) -> Self {
        let direction = value.text_direction();
        let locale = (!value.application_level).then(|| value.locale.clone());
        let scope = Widget::environment_scope(
            (value.locale, value.catalog),
            Directionality::new(direction, value.child).into(),
        );
        if let Some(locale) = locale {
            crate::Semantics::new(scope)
                .container(true)
                .locale_for_subtree(locale)
                .into()
        } else {
            scope
        }
    }
}

/// Ambient selection colors for editable text descendants.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct DefaultSelectionStyle {
    #[builder(default, setter(strip_option))]
    cursor_color: Option<Color>,
    #[builder(default, setter(strip_option))]
    selection_color: Option<Color>,
    #[builder(default, setter(strip_option))]
    handle_color: Option<Color>,
    #[builder(setter(into))]
    child: Widget,
}

impl DefaultSelectionStyle {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            cursor_color: None,
            selection_color: None,
            handle_color: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn cursor_color(mut self, color: Color) -> Self {
        self.cursor_color = Some(color);
        self
    }

    #[must_use]
    pub fn selection_color(mut self, color: Color) -> Self {
        self.selection_color = Some(color);
        self
    }

    #[must_use]
    pub fn handle_color(mut self, color: Color) -> Self {
        self.handle_color = Some(color);
        self
    }
}

impl From<DefaultSelectionStyle> for Widget {
    fn from(value: DefaultSelectionStyle) -> Self {
        let style = (
            value.cursor_color,
            value.selection_color,
            value.handle_color,
        );
        Widget::environment_scope(style, value.child)
    }
}

/// Directionality provides ambient text direction (`Ltr` or `Rtl`) to its subtree.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Directionality {
    text_direction: TextDirection,
    #[builder(setter(into))]
    child: Widget,
}

impl Directionality {
    #[must_use]
    pub fn new(text_direction: TextDirection, child: impl Into<Widget>) -> Self {
        Self {
            text_direction,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn text_direction(&self) -> TextDirection {
        self.text_direction
    }
}

impl From<Directionality> for Widget {
    fn from(value: Directionality) -> Self {
        Widget::environment_scope(value.text_direction, value.child)
    }
}

/// Sets the default [`TextStyle`] and paragraph policy for descendant
/// [`Text`](crate::Text) widgets (Flutter `DefaultTextStyle`). A `Text` that
/// states its own alignment, wrapping, overflow or line cap keeps it; the rest
/// inherit from the nearest ancestor. The paragraph options are not merged
/// across nested `DefaultTextStyle` widgets; the nearest one applies.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct DefaultTextStyle {
    pub(crate) style: TextStyle,
    #[builder(default, setter(into))]
    pub(crate) paragraph: DefaultTextParagraph,
    #[builder(setter(into))]
    pub(crate) child: Widget,
}

/// The paragraph options an ancestor [`DefaultTextStyle`] supplies to `Text`.
/// Environment value of the paragraph options; `None` leaves the built-in default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultTextParagraph {
    pub(crate) align: Option<TextAlign>,
    pub(crate) soft_wrap: Option<bool>,
    pub(crate) max_lines: Option<usize>,
    pub(crate) overflow: Option<TextOverflow>,
}

impl DefaultTextStyle {
    #[must_use]
    pub fn new(style: TextStyle, child: impl Into<Widget>) -> Self {
        Self {
            style,
            paragraph: DefaultTextParagraph::default(),
            child: child.into(),
        }
    }

    /// Flutter `DefaultTextStyle.textAlign`.
    #[must_use]
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.paragraph.align = Some(align);
        self
    }

    /// Flutter `DefaultTextStyle.softWrap`.
    #[must_use]
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.paragraph.soft_wrap = Some(soft_wrap);
        self
    }

    /// Flutter `DefaultTextStyle.overflow`.
    #[must_use]
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.paragraph.overflow = Some(overflow);
        self
    }

    /// Flutter `DefaultTextStyle.maxLines`: a line cap for descendant `Text`
    /// that does not state its own.
    #[must_use]
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.paragraph.max_lines = Some(max_lines);
        self
    }

    #[must_use]
    pub fn style(&self) -> &TextStyle {
        &self.style
    }

    /// Sets every paragraph option at once; used by the animated variant.
    #[must_use]
    pub(crate) fn with_paragraph(mut self, paragraph: DefaultTextParagraph) -> Self {
        self.paragraph = paragraph;
        self
    }
}

impl From<DefaultTextStyle> for Widget {
    fn from(value: DefaultTextStyle) -> Self {
        Widget::environment_scope(
            value.paragraph,
            Widget::environment_scope(value.style, value.child),
        )
    }
}

/// Flutter's `DefaultTextHeightBehavior`: the [`TextHeightBehavior`] of
/// descendant [`Text`](crate::Text) widgets that set none.
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultTextHeightBehavior {
    behavior: TextHeightBehavior,
    child: Widget,
}

impl DefaultTextHeightBehavior {
    #[must_use]
    pub fn new(behavior: TextHeightBehavior, child: impl Into<Widget>) -> Self {
        Self {
            behavior,
            child: child.into(),
        }
    }

    /// `DefaultTextHeightBehavior.maybeOf`.
    #[must_use]
    pub fn maybe_of(context: &BuildContext<'_>) -> Option<TextHeightBehavior> {
        context.depend_on::<TextHeightBehavior>()
    }
}

impl From<DefaultTextHeightBehavior> for Widget {
    fn from(value: DefaultTextHeightBehavior) -> Self {
        Widget::environment_scope(value.behavior, value.child)
    }
}

/// Defines default color, size, and opacity for descendant [`Icon`](crate::Icon) widgets.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct IconTheme {
    #[builder(default, setter(strip_option))]
    color: Option<Color>,
    #[builder(default, setter(transform = |size: f32| Some(size.max(0.0))))]
    size: Option<f32>,
    #[builder(default, setter(transform = |opacity: f32| Some(opacity.clamp(0.0, 1.0))))]
    opacity: Option<f32>,
    #[builder(setter(into))]
    child: Widget,
}

impl IconTheme {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            color: None,
            size: None,
            opacity: None,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size.max(0.0));
        self
    }

    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity.clamp(0.0, 1.0));
        self
    }
}

impl IconTheme {
    /// The size this theme sets, if any.
    pub(crate) const fn icon_size(&self) -> Option<f32> {
        self.size
    }
}

impl From<IconTheme> for Widget {
    fn from(value: IconTheme) -> Self {
        let child = value.child.clone();
        Widget::environment_scope(value, child)
    }
}

/// Device / viewport orientation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Builds a widget subtree depending on the parent's orientation.
type OrientationCallback = Rc<dyn for<'a> Fn(&BuildContext<'a>, Orientation) -> Widget>;

#[derive(Clone)]
pub struct OrientationBuilder {
    builder: OrientationCallback,
}

impl OrientationBuilder {
    #[must_use]
    pub fn new<W>(builder: impl for<'a> Fn(&BuildContext<'a>, Orientation) -> W + 'static) -> Self
    where
        W: Into<Widget> + 'static,
    {
        Self {
            builder: Rc::new(move |ctx, orient| builder(ctx, orient).into()),
        }
    }
}

impl From<OrientationBuilder> for Widget {
    fn from(value: OrientationBuilder) -> Self {
        let builder = value.builder;
        LayoutBuilder::new(move |context, constraints| {
            let orientation = if constraints.max_width() > constraints.max_height() {
                Orientation::Landscape
            } else {
                Orientation::Portrait
            };
            builder(context, orientation)
        })
        .into()
    }
}

/// Flutter's `ScrollConfiguration`: the [`ScrollBehavior`] descendant scroll
/// views follow for physics they do not set, drag devices, scrollbars and
/// velocity tracking.
///
/// [`ScrollBehavior`]: crate::ScrollBehavior
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct ScrollConfiguration {
    #[builder(default)]
    behavior: crate::ScrollBehavior,
    #[builder(setter(into))]
    child: Widget,
}

impl ScrollConfiguration {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            behavior: crate::ScrollBehavior::new(),
            child: child.into(),
        }
    }

    #[must_use]
    pub fn behavior(mut self, behavior: crate::ScrollBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    /// The nearest inherited scroll behavior, or the platform defaults if
    /// no ancestor supplied one. A build that reads this value tracks it.
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> crate::ScrollBehavior {
        context.depend_on::<crate::ScrollBehavior>().unwrap_or_default()
    }

    /// Shorthand for a behavior whose physics override the platform's.
    #[must_use]
    pub fn physics(mut self, physics: ScrollPhysics) -> Self {
        self.behavior = self.behavior.physics(physics);
        self
    }

    #[must_use]
    pub fn get_behavior(&self) -> &crate::ScrollBehavior {
        &self.behavior
    }
}

impl From<ScrollConfiguration> for Widget {
    fn from(value: ScrollConfiguration) -> Self {
        Widget::environment_scope(value.behavior, value.child)
    }
}

/// Associates a [`ScrollController`] with the subtree as the default primary scroll controller.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct PrimaryScrollController {
    controller: ScrollController,
    #[builder(default = true, setter(skip))]
    automatically_inherit_for_platforms: bool,
    #[builder(setter(into))]
    child: Widget,
}

impl PrimaryScrollController {
    #[must_use]
    pub fn new(controller: ScrollController, child: impl Into<Widget>) -> Self {
        Self {
            controller,
            automatically_inherit_for_platforms: true,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn controller(&self) -> &ScrollController {
        &self.controller
    }
}

impl From<PrimaryScrollController> for Widget {
    fn from(value: PrimaryScrollController) -> Self {
        Widget::environment_scope(value.controller, value.child)
    }
}

/// Flutter's `TickerMode`: enables or disables the animation tickers of its
/// subtree. A disabled subtree's controllers keep time but stop ticking, so
/// they jump to the current value when enabled again. Nested modes combine:
/// a subtree ticks only if every enclosing mode is enabled.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct TickerMode {
    enabled: bool,
    #[builder(setter(into))]
    child: Widget,
}

impl TickerMode {
    #[must_use]
    pub fn new(enabled: bool, child: impl Into<Widget>) -> Self {
        Self {
            enabled,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Flutter `TickerMode.of`: whether tickers under `context` are enabled.
    /// Controllers created with `with_vsync` follow it through
    /// `set_ticker_muted(!TickerMode::of(context))`.
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> bool {
        context.ticker_mode_enabled()
    }
}

/// The local ticker gate published by an explicit TickerMode or Visibility.
/// A descendant can only tick when every gate is enabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TickerModeScope(pub(crate) bool);

impl From<TickerMode> for Widget {
    fn from(value: TickerMode) -> Self {
        Widget::environment_scope(TickerModeScope(value.enabled), value.child)
    }
}

/// Tracks the sensitivity requests contributed by a retained widget subtree.
///
/// Flutter resolves multiple [`SensitiveContent`] widgets by priority rather
/// than by nearest-ancestor shadowing: any `Sensitive` request wins, then any
/// `AutoSensitive` request, then `NotSensitive`.  The runtime takes a snapshot
/// of this value at its window boundary and emits one normalized platform
/// command when that snapshot changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SensitiveContentHost {
    sensitive_count: usize,
    auto_sensitive_count: usize,
    not_sensitive_count: usize,
    fallback: ContentSensitivity,
}

impl Default for SensitiveContentHost {
    fn default() -> Self {
        Self::new()
    }
}

impl SensitiveContentHost {
    /// Creates an empty host whose neutral fallback is `NotSensitive`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sensitive_count: 0,
            auto_sensitive_count: 0,
            not_sensitive_count: 0,
            fallback: ContentSensitivity::NotSensitive,
        }
    }

    /// Creates an empty host with an explicit policy to restore after its last
    /// sensitive-content registration is removed.
    #[must_use]
    pub const fn with_fallback(fallback: ContentSensitivity) -> Self {
        Self {
            fallback,
            ..Self::new()
        }
    }

    /// Registers one retained [`SensitiveContent`] request.
    pub fn register(&mut self, sensitivity: ContentSensitivity) {
        match sensitivity {
            ContentSensitivity::Sensitive => self.sensitive_count += 1,
            ContentSensitivity::AutoSensitive => self.auto_sensitive_count += 1,
            ContentSensitivity::NotSensitive => self.not_sensitive_count += 1,
        }
    }

    /// Removes one request. Returns `false` when the corresponding count was
    /// already empty, keeping stale lifecycle notifications harmless.
    pub fn unregister(&mut self, sensitivity: ContentSensitivity) -> bool {
        let count = match sensitivity {
            ContentSensitivity::Sensitive => &mut self.sensitive_count,
            ContentSensitivity::AutoSensitive => &mut self.auto_sensitive_count,
            ContentSensitivity::NotSensitive => &mut self.not_sensitive_count,
        };
        if *count == 0 {
            return false;
        }
        *count -= 1;
        true
    }

    /// Returns the highest-priority request currently registered, or `None`
    /// when no [`SensitiveContent`] widget is mounted.
    #[must_use]
    pub const fn calculated_content_sensitivity(&self) -> Option<ContentSensitivity> {
        if self.sensitive_count > 0 {
            Some(ContentSensitivity::Sensitive)
        } else if self.auto_sensitive_count > 0 {
            Some(ContentSensitivity::AutoSensitive)
        } else if self.not_sensitive_count > 0 {
            Some(ContentSensitivity::NotSensitive)
        } else {
            None
        }
    }

    /// Returns the policy that should be active at a window boundary,
    /// including the configured fallback when no widget is registered.
    #[must_use]
    pub const fn effective_content_sensitivity(&self) -> ContentSensitivity {
        match self.calculated_content_sensitivity() {
            Some(sensitivity) => sensitivity,
            None => self.fallback,
        }
    }

    /// Returns whether this host currently tracks at least one widget.
    #[must_use]
    pub const fn has_widgets(&self) -> bool {
        self.sensitive_count > 0 || self.auto_sensitive_count > 0 || self.not_sensitive_count > 0
    }

    /// Returns the number of registrations for one sensitivity value.
    #[must_use]
    pub const fn count(&self, sensitivity: ContentSensitivity) -> usize {
        match sensitivity {
            ContentSensitivity::Sensitive => self.sensitive_count,
            ContentSensitivity::AutoSensitive => self.auto_sensitive_count,
            ContentSensitivity::NotSensitive => self.not_sensitive_count,
        }
    }
}

/// Stops retained typed-environment lookup at this subtree boundary.
///
/// The retained equivalent of Flutter's static `LookupBoundary` methods is
/// [`LookupBoundary::lookup`]. Ordinary environment scopes nested inside the
/// boundary remain visible to descendants; only scopes outside the nearest
/// boundary are hidden.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct LookupBoundary {
    #[builder(setter(into))]
    child: Widget,
}

impl LookupBoundary {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }

    #[must_use]
    pub fn child(&self) -> &Widget {
        &self.child
    }

    /// Reads a typed environment value visible from the currently materialized
    /// retained builder. A lookup never crosses the nearest boundary.
    #[must_use]
    pub fn lookup<T: Any + Clone>(context: &BuildContext<'_>) -> Option<T> {
        context.find::<T>()
    }
}

impl From<LookupBoundary> for Widget {
    fn from(value: LookupBoundary) -> Self {
        Widget::environment_boundary(value.child)
    }
}

/// Marks content as sensitive to obscure it from window sharing, screen recording, and diagnostics.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct SensitiveContent {
    #[builder(default = ContentSensitivity::Sensitive)]
    sensitivity: ContentSensitivity,
    #[builder(setter(into))]
    child: Widget,
}

impl SensitiveContent {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            sensitivity: ContentSensitivity::Sensitive,
            child: child.into(),
        }
    }

    /// Creates a scope with Flutter's explicit sensitivity policy.
    #[must_use]
    pub fn with_sensitivity(sensitivity: ContentSensitivity, child: impl Into<Widget>) -> Self {
        Self {
            sensitivity,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn sensitivity(&self) -> ContentSensitivity {
        self.sensitivity
    }

    /// Compatibility setter for the original Incular boolean surface.
    /// `true` maps to `Sensitive`; `false` maps to `NotSensitive`.
    #[must_use]
    pub fn sensitive(mut self, sensitive: bool) -> Self {
        self.sensitivity = if sensitive {
            ContentSensitivity::Sensitive
        } else {
            ContentSensitivity::NotSensitive
        };
        self
    }
}

impl From<SensitiveContent> for Widget {
    fn from(value: SensitiveContent) -> Self {
        Widget::environment_scope(value.sensitivity, value.child)
    }
}
