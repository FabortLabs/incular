//! Build-time inherited environment access.
//!
//! This module owns borrowed build context views and inherited descriptor values; retained dependency tracking remains owned by `WidgetTree`.

use super::*;
use incular_config::{
    Brightness, DisplayFeature, InputCapabilities, Locale, LocaleResolver, RuntimeEnvironment,
    TextDirection,
};

#[derive(Clone)]
struct RuntimeEnvironmentSnapshot(RuntimeEnvironment);

#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeViewport(Size);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeScaleFactor(f64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeBrightness(Brightness);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RuntimeTextScale(pub(super) f32);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RuntimeSafeInsets(pub(super) EdgeInsets);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeViewPadding(EdgeInsets);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeViewInsets(EdgeInsets);
#[derive(Clone, Debug, PartialEq)]
struct RuntimeLocales(Vec<Locale>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeReducedMotion(bool);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeHighContrast(bool);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeInputCapabilities(InputCapabilities);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeWindowFocused(bool);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeWindowOccluded(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeSystemGestureInsets(EdgeInsets);
#[derive(Clone, Debug, PartialEq)]
struct RuntimeDisplayFeatures(Vec<DisplayFeature>);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeAlwaysUse24HourFormat(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeAccessibleNavigation(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeInvertColors(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeOnOffSwitchLabels(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RuntimeBoldText(pub(super) bool);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeSupportsAnnounce(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeNavigationMode(incular_config::NavigationMode);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeSupportsShowingSystemContextMenu(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RuntimeLineHeightScaleFactorOverride(pub(super) Option<f32>);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RuntimeLetterSpacingOverride(pub(super) Option<f32>);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RuntimeWordSpacingOverride(pub(super) Option<f32>);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeParagraphSpacingOverride(Option<f32>);

// Flutter's `MediaQuery.xOf` aspects. Each environment scope that installs a
// `RuntimeEnvironment` also installs one of these per field, so a reader
// depends only on the field it reads and is not invalidated by changes to the
// others. They are distinct from the `Runtime*` window values, so
// `BuildContext::viewport` keeps ignoring `MediaQuery` overrides.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaQuerySize(Size);
#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaQueryWidth(f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaQueryHeight(f32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MediaQueryOrientation(crate::Orientation);

fn orientation(size: Size) -> crate::Orientation {
    if size.width > size.height {
        crate::Orientation::Landscape
    } else {
        crate::Orientation::Portrait
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaQueryDevicePixelRatio(f64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MediaQueryTextScale(pub(super) f32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MediaQueryPlatformBrightness(Brightness);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MediaQueryDisableAnimations(bool);
#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaQueryPadding(EdgeInsets);
#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaQueryViewPadding(EdgeInsets);
#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaQueryViewInsets(EdgeInsets);

macro_rules! media_query_aspects {
    ($($wrapper:ident from $field:ident),* $(,)?) => {
        /// Installs every aspect of `environment` into `context`, which must
        /// be the fresh environment of a `RuntimeEnvironment` scope.
        fn install_media_query_aspects(context: &DependencyContext, environment: &RuntimeEnvironment) {
            context.set_erased(TypeId::of::<MediaQueryWidth>(), Rc::new(MediaQueryWidth(environment.viewport.width)));
            context.set_erased(TypeId::of::<MediaQueryHeight>(), Rc::new(MediaQueryHeight(environment.viewport.height)));
            context.set_erased(TypeId::of::<MediaQueryOrientation>(), Rc::new(MediaQueryOrientation(orientation(environment.viewport))));
            $(
                context.set_erased(
                    TypeId::of::<$wrapper>(),
                    Rc::new($wrapper(environment.$field.clone())),
                );
            )*
        }

        /// Replaces only the aspects whose field changed between two snapshots.
        fn update_media_query_aspects(
            context: &DependencyContext,
            previous: &RuntimeEnvironment,
            next: &RuntimeEnvironment,
        ) {
            if previous.viewport.width != next.viewport.width {
                context.set_erased(TypeId::of::<MediaQueryWidth>(), Rc::new(MediaQueryWidth(next.viewport.width)));
            }
            if previous.viewport.height != next.viewport.height {
                context.set_erased(TypeId::of::<MediaQueryHeight>(), Rc::new(MediaQueryHeight(next.viewport.height)));
            }
            if orientation(previous.viewport) != orientation(next.viewport) {
                context.set_erased(TypeId::of::<MediaQueryOrientation>(), Rc::new(MediaQueryOrientation(orientation(next.viewport))));
            }
            $(
                if previous.$field != next.$field {
                    context.set_erased(
                        TypeId::of::<$wrapper>(),
                        Rc::new($wrapper(next.$field.clone())),
                    );
                }
            )*
        }
    };
}

media_query_aspects! {
    MediaQuerySize from viewport,
    MediaQueryDevicePixelRatio from scale_factor,
    MediaQueryTextScale from text_scale,
    MediaQueryPlatformBrightness from brightness,
    MediaQueryDisableAnimations from reduced_motion,
    MediaQueryPadding from safe_insets,
    MediaQueryViewPadding from view_padding,
    MediaQueryViewInsets from view_insets,
    RuntimeSafeInsets from safe_insets,
    RuntimeViewPadding from view_padding,
    RuntimeViewInsets from view_insets,
    RuntimeHighContrast from high_contrast,
    RuntimeSystemGestureInsets from system_gesture_insets,
    RuntimeDisplayFeatures from display_features,
    RuntimeAlwaysUse24HourFormat from always_use_24_hour_format,
    RuntimeAccessibleNavigation from accessible_navigation,
    RuntimeInvertColors from invert_colors,
    RuntimeOnOffSwitchLabels from on_off_switch_labels,
    RuntimeBoldText from bold_text,
    RuntimeSupportsAnnounce from supports_announce,
    RuntimeNavigationMode from navigation_mode,
    RuntimeSupportsShowingSystemContextMenu from supports_showing_system_context_menu,
    RuntimeLineHeightScaleFactorOverride from line_height_scale_factor_override,
    RuntimeLetterSpacingOverride from letter_spacing_override,
    RuntimeWordSpacingOverride from word_spacing_override,
    RuntimeParagraphSpacingOverride from paragraph_spacing_override,
}

/// Installs separately observed fields of a runtime or Visibility scope.
pub(super) fn install_scope_aspects(context: &DependencyContext, scope: &InheritedScopeValue) {
    if let Some(environment) = scope.value.downcast_ref::<RuntimeEnvironment>() {
        install_media_query_aspects(context, environment);
    } else if let Some(scope) = scope.value.downcast_ref::<VisibilityScope>() {
        context.set_erased(
            TypeId::of::<VisibilityObservation>(),
            Rc::new(VisibilityObservation(scope.visible)),
        );
        context.set_erased(
            TypeId::of::<crate::environment::TickerModeScope>(),
            Rc::new(crate::environment::TickerModeScope(scope.ticks_enabled)),
        );
    }
}

/// Publishes only the observed fields whose values actually changed.
pub(super) fn update_scope_aspects(
    context: &DependencyContext,
    previous: &InheritedScopeValue,
    next: &InheritedScopeValue,
) {
    if let (Some(previous), Some(next)) = (
        previous.value.downcast_ref::<RuntimeEnvironment>(),
        next.value.downcast_ref::<RuntimeEnvironment>(),
    ) {
        update_media_query_aspects(context, previous, next);
    } else if let (Some(previous), Some(next)) = (
        previous.value.downcast_ref::<VisibilityScope>(),
        next.value.downcast_ref::<VisibilityScope>(),
    ) {
        if previous.visible != next.visible {
            context.set_erased(
                TypeId::of::<VisibilityObservation>(),
                Rc::new(VisibilityObservation(next.visible)),
            );
        }
        if previous.ticks_enabled != next.ticks_enabled {
            context.set_erased(
                TypeId::of::<crate::environment::TickerModeScope>(),
                Rc::new(crate::environment::TickerModeScope(next.ticks_enabled)),
            );
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VisibilityScope {
    pub(super) visible: bool,
    pub(super) ticks_enabled: bool,
}

#[derive(Clone, Copy)]
pub(super) struct VisibilityObservation(pub(super) bool);

impl WidgetTree {
    /// Reads the mounted editor's effective platform menu capability, including
    /// an inherited MediaQuery override. A read-only field cannot use this route.
    #[doc(hidden)]
    pub fn system_context_menu_field_eligible(&self, id: ElementId) -> bool {
        let Some(element) = self.elements.get(id.0) else {
            return false;
        };
        matches!(element.widget.kind(), WidgetKind::TextField(spec) if spec.enabled && !spec.read_only)
            && element.build_context
                .read::<RuntimeSupportsShowingSystemContextMenu>()
                .is_some_and(|value| value.0)
    }

    /// Reads an application catalog message at a mounted element without
    /// introducing a retained build dependency.
    #[doc(hidden)]
    pub fn localized_text_at(&self, id: ElementId, key: &str) -> Option<String> {
        use incular_config::{Locale, LocaleResolver, LocalizationCatalog};

        let context = &self.elements.get(id.0)?.build_context;
        let scope = context.read_shared::<(Locale, Rc<dyn LocalizationCatalog>)>()?;
        LocaleResolver::resolve_message(std::slice::from_ref(&scope.0), scope.1.as_ref(), key)
            .map(|message| message.value.to_owned())
    }
}

/// A borrowed view of the retained inherited environment for one live build
/// operation. The reference cannot escape the callback that receives it.
pub struct BuildContext<'a> {
    inner: &'a DependencyContext,
}

pub(crate) type LayoutBuilderCallback = dyn for<'a> Fn(&BuildContext<'a>, Constraints) -> Widget;

impl<'a> BuildContext<'a> {
    pub(super) fn new(inner: &'a DependencyContext) -> Self {
        Self { inner }
    }

    /// Reads the nearest inherited value and registers this element as a
    /// dependent of the exact retained scope that supplied it.
    #[must_use]
    pub fn depend_on<T: Any + Clone>(&self) -> Option<T> {
        self.inner.depend::<T>()
    }

    /// Reads the nearest inherited value without creating an invalidation edge.
    #[must_use]
    pub fn find<T: Any + Clone>(&self) -> Option<T> {
        self.inner.read::<T>()
    }

    /// Shared-ownership form of [`BuildContext::depend_on`].
    #[must_use]
    pub fn depend_on_shared<T: Any>(&self) -> Option<Rc<T>> {
        self.inner.depend_shared::<T>()
    }

    /// Shared-ownership form of [`BuildContext::find`].
    #[must_use]
    pub fn find_shared<T: Any>(&self) -> Option<Rc<T>> {
        self.inner.read_shared::<T>()
    }

    pub(crate) fn visibility(&self) -> bool {
        self.inner
            .all_retained_inherited::<VisibilityObservation>(|scope| scope.0)
    }

    pub(crate) fn ticker_mode_enabled(&self) -> bool {
        self.inner
            .all_inherited_filtered::<crate::environment::TickerModeScope, _>(|scope| scope.0)
    }

    /// The window's operating-system sound service, when mounted in a runtime.
    #[must_use]
    pub fn system_sound(&self) -> Option<incular_platform::SystemSound> {
        self.find()
    }

    /// Current logical viewport from the native/runtime environment. This is
    /// independent of inherited `MediaQuery` overrides and subscribes only to
    /// viewport changes in the owning retained window.
    #[must_use]
    pub fn viewport(&self) -> Size {
        self.inner
            .depend::<RuntimeViewport>()
            .map_or(Size::ZERO, |value| value.0)
    }

    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.inner
            .depend::<RuntimeScaleFactor>()
            .map_or(1.0, |value| value.0)
    }

    #[must_use]
    pub fn brightness(&self) -> Brightness {
        self.inner
            .depend::<RuntimeBrightness>()
            .map_or(Brightness::Light, |value| value.0)
    }

    #[must_use]
    pub fn text_scale(&self) -> f32 {
        self.inner
            .depend::<RuntimeTextScale>()
            .map_or(1.0, |value| value.0)
    }

    #[must_use]
    pub fn safe_insets(&self) -> EdgeInsets {
        self.inner
            .depend::<RuntimeSafeInsets>()
            .map_or(EdgeInsets::ZERO, |value| value.0)
    }

    #[must_use]
    pub fn view_padding(&self) -> EdgeInsets {
        self.inner
            .depend::<RuntimeViewPadding>()
            .map_or(EdgeInsets::ZERO, |value| value.0)
    }

    #[must_use]
    pub fn view_insets(&self) -> EdgeInsets {
        self.inner
            .depend::<RuntimeViewInsets>()
            .map_or(EdgeInsets::ZERO, |value| value.0)
    }

    #[must_use]
    pub fn locales(&self) -> Vec<Locale> {
        self.inner
            .depend::<RuntimeLocales>()
            .map_or_else(Vec::new, |value| value.0)
    }

    #[must_use]
    pub fn resolve_locale(&self, supported: &[Locale]) -> Option<Locale> {
        LocaleResolver::resolve(&self.locales(), supported)
    }

    /// The text direction supplied by the nearest ancestor
    /// [`Directionality`](crate::Directionality), or `None` when there is none
    /// (Flutter's `Directionality.maybeOf`).
    ///
    /// There is deliberately no implicit left-to-right default: a widget that
    /// needs a direction and finds none is a build error
    /// (`TreeError::MissingDirectionality`), as in Flutter.
    #[must_use]
    pub fn text_direction(&self) -> Option<TextDirection> {
        self.inner.depend::<TextDirection>()
    }

    #[must_use]
    pub fn reduced_motion(&self) -> bool {
        self.inner
            .depend::<RuntimeReducedMotion>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn high_contrast(&self) -> bool {
        self.inner
            .depend::<RuntimeHighContrast>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn input_capabilities(&self) -> InputCapabilities {
        self.inner
            .depend::<RuntimeInputCapabilities>()
            .map_or_else(InputCapabilities::default, |value| value.0)
    }

    #[must_use]
    pub fn window_focused(&self) -> bool {
        self.inner
            .depend::<RuntimeWindowFocused>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn window_occluded(&self) -> bool {
        self.inner
            .depend::<RuntimeWindowOccluded>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn system_gesture_insets(&self) -> EdgeInsets {
        self.inner
            .depend::<RuntimeSystemGestureInsets>()
            .map_or(EdgeInsets::ZERO, |value| value.0)
    }

    /// Flutter's `MediaQuery.displayFeaturesOf`: hinges, folds and cutouts
    /// in view coordinates. A [`DisplayFeatureSubScreen`](crate::DisplayFeatureSubScreen)
    /// keeps only those overlapping its sub-screen.
    ///
    /// Host support: the desktop and browser hosts report none, as
    /// Flutter's do, since neither platform exposes hinges or folds to a
    /// window. An Android host fills `RuntimeEnvironment::display_features`
    /// from Jetpack WindowManager in its own activity wiring, which is
    /// outside `incular-android`. Without them the list is empty and
    /// sub-screens span the whole view.
    #[must_use]
    pub fn display_features(&self) -> Vec<DisplayFeature> {
        self.inner
            .depend::<RuntimeDisplayFeatures>()
            .map_or_else(Vec::new, |value| value.0)
    }

    #[must_use]
    pub fn always_use_24_hour_format(&self) -> bool {
        self.inner
            .depend::<RuntimeAlwaysUse24HourFormat>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn accessible_navigation(&self) -> bool {
        self.inner
            .depend::<RuntimeAccessibleNavigation>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn invert_colors(&self) -> bool {
        self.inner
            .depend::<RuntimeInvertColors>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn on_off_switch_labels(&self) -> bool {
        self.inner
            .depend::<RuntimeOnOffSwitchLabels>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn bold_text(&self) -> bool {
        self.inner
            .depend::<RuntimeBoldText>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn supports_announce(&self) -> bool {
        self.inner
            .depend::<RuntimeSupportsAnnounce>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn navigation_mode(&self) -> incular_config::NavigationMode {
        self.inner
            .depend::<RuntimeNavigationMode>()
            .map_or(incular_config::NavigationMode::Traditional, |value| value.0)
    }

    #[must_use]
    pub fn supports_showing_system_context_menu(&self) -> bool {
        self.inner
            .depend::<RuntimeSupportsShowingSystemContextMenu>()
            .is_some_and(|value| value.0)
    }

    #[must_use]
    pub fn line_height_scale_factor_override(&self) -> Option<f32> {
        self.inner
            .depend::<RuntimeLineHeightScaleFactorOverride>()
            .and_then(|value| value.0)
    }

    #[must_use]
    pub fn letter_spacing_override(&self) -> Option<f32> {
        self.inner
            .depend::<RuntimeLetterSpacingOverride>()
            .and_then(|value| value.0)
    }

    #[must_use]
    pub fn word_spacing_override(&self) -> Option<f32> {
        self.inner
            .depend::<RuntimeWordSpacingOverride>()
            .and_then(|value| value.0)
    }

    #[must_use]
    pub fn paragraph_spacing_override(&self) -> Option<f32> {
        self.inner
            .depend::<RuntimeParagraphSpacingOverride>()
            .and_then(|value| value.0)
    }

    /// Flutter's `MediaQueryData.disableAnimations`, which Incular reads
    /// from the reduced-motion preference.
    #[must_use]
    pub fn disable_animations(&self) -> bool {
        self.reduced_motion()
    }

    /// Reads a complete snapshot for hosts that override only a subset of
    /// the inherited MediaQuery data.
    pub(crate) fn effective_media_query(&self) -> RuntimeEnvironment {
        self.depend_on::<RuntimeEnvironment>().unwrap_or_else(|| {
            self.depend_on::<RuntimeEnvironmentSnapshot>()
                .map(|snapshot| snapshot.0)
                .unwrap_or_default()
        })
    }

    /// Flutter's `MediaQuery.sizeOf`: the size of the nearest
    /// [`MediaQuery`](crate::MediaQuery) (or application view) scope. Depends
    /// only on the size aspect, so other environment changes do not rebuild
    /// the reader. `None` outside any environment scope.
    #[must_use]
    pub fn media_query_size(&self) -> Option<Size> {
        self.inner.depend::<MediaQuerySize>().map(|value| value.0)
    }

    #[must_use]
    pub fn media_query_width(&self) -> Option<f32> {
        self.inner.depend::<MediaQueryWidth>().map(|value| value.0)
    }

    #[must_use]
    pub fn media_query_height(&self) -> Option<f32> {
        self.inner.depend::<MediaQueryHeight>().map(|value| value.0)
    }

    #[must_use]
    pub fn media_query_orientation(&self) -> Option<crate::Orientation> {
        self.inner
            .depend::<MediaQueryOrientation>()
            .map(|value| value.0)
    }

    /// Flutter's `MediaQuery.devicePixelRatioOf`, aspect-scoped like
    /// [`Self::media_query_size`].
    #[must_use]
    pub fn media_query_device_pixel_ratio(&self) -> Option<f64> {
        self.inner
            .depend::<MediaQueryDevicePixelRatio>()
            .map(|value| value.0)
    }

    /// Flutter's `MediaQuery.textScalerOf` (as a scale factor), aspect-scoped.
    #[must_use]
    pub fn media_query_text_scale(&self) -> Option<f32> {
        self.inner
            .depend::<MediaQueryTextScale>()
            .map(|value| value.0)
    }

    /// Flutter's `MediaQuery.platformBrightnessOf`, aspect-scoped.
    #[must_use]
    pub fn media_query_platform_brightness(&self) -> Option<Brightness> {
        self.inner
            .depend::<MediaQueryPlatformBrightness>()
            .map(|value| value.0)
    }

    /// Flutter's scoped `MediaQuery.disableAnimationsOf`. The window's
    /// native preference remains available separately via `reduced_motion`.
    #[must_use]
    pub fn media_query_disable_animations(&self) -> Option<bool> {
        self.inner.depend::<MediaQueryDisableAnimations>().map(|value| value.0)
    }

    /// Flutter's `MediaQuery.paddingOf`, aspect-scoped.
    #[must_use]
    pub fn media_query_padding(&self) -> Option<EdgeInsets> {
        self.inner
            .depend::<MediaQueryPadding>()
            .map(|value| value.0)
    }

    /// Flutter's `MediaQuery.viewPaddingOf`, aspect-scoped.
    #[must_use]
    pub fn media_query_view_padding(&self) -> Option<EdgeInsets> {
        self.inner
            .depend::<MediaQueryViewPadding>()
            .map(|value| value.0)
    }

    /// Flutter's `MediaQuery.viewInsetsOf`, aspect-scoped.
    #[must_use]
    pub fn media_query_view_insets(&self) -> Option<EdgeInsets> {
        self.inner
            .depend::<MediaQueryViewInsets>()
            .map(|value| value.0)
    }
}

pub(super) fn install_runtime_environment(
    context: &DependencyContext,
    environment: &RuntimeEnvironment,
) {
    // Flutter reads `disableAnimations` from the platform's accessibility
    // features; here it follows the window's reduced-motion preference.
    incular_animation::set_disable_animations(environment.reduced_motion);
    context.set_erased(
        TypeId::of::<RuntimeEnvironmentSnapshot>(),
        Rc::new(RuntimeEnvironmentSnapshot(environment.clone())),
    );
    context.set_erased(
        TypeId::of::<RuntimeViewport>(),
        Rc::new(RuntimeViewport(environment.viewport)),
    );
    context.set_erased(
        TypeId::of::<RuntimeScaleFactor>(),
        Rc::new(RuntimeScaleFactor(environment.scale_factor)),
    );
    context.set_erased(
        TypeId::of::<RuntimeBrightness>(),
        Rc::new(RuntimeBrightness(environment.brightness)),
    );
    context.set_erased(
        TypeId::of::<RuntimeTextScale>(),
        Rc::new(RuntimeTextScale(environment.text_scale)),
    );
    context.set_erased(
        TypeId::of::<RuntimeSafeInsets>(),
        Rc::new(RuntimeSafeInsets(environment.safe_insets)),
    );
    context.set_erased(
        TypeId::of::<RuntimeViewPadding>(),
        Rc::new(RuntimeViewPadding(environment.view_padding)),
    );
    context.set_erased(
        TypeId::of::<RuntimeViewInsets>(),
        Rc::new(RuntimeViewInsets(environment.view_insets)),
    );
    context.set_erased(
        TypeId::of::<RuntimeLocales>(),
        Rc::new(RuntimeLocales(environment.locales.clone())),
    );
    context.set_erased(
        TypeId::of::<RuntimeReducedMotion>(),
        Rc::new(RuntimeReducedMotion(environment.reduced_motion)),
    );
    context.set_erased(
        TypeId::of::<RuntimeHighContrast>(),
        Rc::new(RuntimeHighContrast(environment.high_contrast)),
    );
    context.set_erased(
        TypeId::of::<RuntimeInputCapabilities>(),
        Rc::new(RuntimeInputCapabilities(environment.input)),
    );
    context.set_erased(
        TypeId::of::<RuntimeWindowFocused>(),
        Rc::new(RuntimeWindowFocused(environment.window_focused)),
    );
    context.set_erased(
        TypeId::of::<RuntimeWindowOccluded>(),
        Rc::new(RuntimeWindowOccluded(environment.window_occluded)),
    );
    context.set_erased(
        TypeId::of::<RuntimeSystemGestureInsets>(),
        Rc::new(RuntimeSystemGestureInsets(
            environment.system_gesture_insets,
        )),
    );
    context.set_erased(
        TypeId::of::<RuntimeDisplayFeatures>(),
        Rc::new(RuntimeDisplayFeatures(environment.display_features.clone())),
    );
    context.set_erased(
        TypeId::of::<RuntimeAlwaysUse24HourFormat>(),
        Rc::new(RuntimeAlwaysUse24HourFormat(
            environment.always_use_24_hour_format,
        )),
    );
    context.set_erased(
        TypeId::of::<RuntimeAccessibleNavigation>(),
        Rc::new(RuntimeAccessibleNavigation(
            environment.accessible_navigation,
        )),
    );
    context.set_erased(
        TypeId::of::<RuntimeInvertColors>(),
        Rc::new(RuntimeInvertColors(environment.invert_colors)),
    );
    context.set_erased(
        TypeId::of::<RuntimeOnOffSwitchLabels>(),
        Rc::new(RuntimeOnOffSwitchLabels(environment.on_off_switch_labels)),
    );
    context.set_erased(
        TypeId::of::<RuntimeBoldText>(),
        Rc::new(RuntimeBoldText(environment.bold_text)),
    );
    context.set_erased(
        TypeId::of::<RuntimeSupportsAnnounce>(),
        Rc::new(RuntimeSupportsAnnounce(environment.supports_announce)),
    );
    context.set_erased(
        TypeId::of::<RuntimeNavigationMode>(),
        Rc::new(RuntimeNavigationMode(environment.navigation_mode)),
    );
    context.set_erased(
        TypeId::of::<RuntimeSupportsShowingSystemContextMenu>(),
        Rc::new(RuntimeSupportsShowingSystemContextMenu(
            environment.supports_showing_system_context_menu,
        )),
    );
    context.set_erased(
        TypeId::of::<RuntimeLineHeightScaleFactorOverride>(),
        Rc::new(RuntimeLineHeightScaleFactorOverride(
            environment.line_height_scale_factor_override,
        )),
    );
    context.set_erased(
        TypeId::of::<RuntimeLetterSpacingOverride>(),
        Rc::new(RuntimeLetterSpacingOverride(
            environment.letter_spacing_override,
        )),
    );
    context.set_erased(
        TypeId::of::<RuntimeWordSpacingOverride>(),
        Rc::new(RuntimeWordSpacingOverride(
            environment.word_spacing_override,
        )),
    );
    context.set_erased(
        TypeId::of::<RuntimeParagraphSpacingOverride>(),
        Rc::new(RuntimeParagraphSpacingOverride(
            environment.paragraph_spacing_override,
        )),
    );
}

pub(super) fn update_runtime_environment(
    context: &DependencyContext,
    previous: &RuntimeEnvironment,
    next: &RuntimeEnvironment,
) {
    macro_rules! replace_if_changed {
        ($field:ident, $wrapper:ident) => {
            if previous.$field != next.$field {
                context.set_erased(
                    TypeId::of::<$wrapper>(),
                    Rc::new($wrapper(next.$field.clone())),
                );
            }
        };
    }

    if previous != next {
        context.set_erased(
            TypeId::of::<RuntimeEnvironmentSnapshot>(),
            Rc::new(RuntimeEnvironmentSnapshot(next.clone())),
        );
    }
    replace_if_changed!(viewport, RuntimeViewport);
    replace_if_changed!(scale_factor, RuntimeScaleFactor);
    replace_if_changed!(brightness, RuntimeBrightness);
    replace_if_changed!(text_scale, RuntimeTextScale);
    replace_if_changed!(safe_insets, RuntimeSafeInsets);
    replace_if_changed!(view_padding, RuntimeViewPadding);
    replace_if_changed!(view_insets, RuntimeViewInsets);
    replace_if_changed!(locales, RuntimeLocales);
    replace_if_changed!(reduced_motion, RuntimeReducedMotion);
    if previous.reduced_motion != next.reduced_motion {
        incular_animation::set_disable_animations(next.reduced_motion);
    }
    replace_if_changed!(high_contrast, RuntimeHighContrast);
    replace_if_changed!(input, RuntimeInputCapabilities);
    replace_if_changed!(window_focused, RuntimeWindowFocused);
    replace_if_changed!(window_occluded, RuntimeWindowOccluded);
    replace_if_changed!(system_gesture_insets, RuntimeSystemGestureInsets);
    replace_if_changed!(display_features, RuntimeDisplayFeatures);
    replace_if_changed!(always_use_24_hour_format, RuntimeAlwaysUse24HourFormat);
    replace_if_changed!(accessible_navigation, RuntimeAccessibleNavigation);
    replace_if_changed!(invert_colors, RuntimeInvertColors);
    replace_if_changed!(on_off_switch_labels, RuntimeOnOffSwitchLabels);
    replace_if_changed!(bold_text, RuntimeBoldText);
    replace_if_changed!(supports_announce, RuntimeSupportsAnnounce);
    replace_if_changed!(navigation_mode, RuntimeNavigationMode);
    replace_if_changed!(
        supports_showing_system_context_menu,
        RuntimeSupportsShowingSystemContextMenu
    );
    replace_if_changed!(
        line_height_scale_factor_override,
        RuntimeLineHeightScaleFactorOverride
    );
    replace_if_changed!(letter_spacing_override, RuntimeLetterSpacingOverride);
    replace_if_changed!(word_spacing_override, RuntimeWordSpacingOverride);
    replace_if_changed!(paragraph_spacing_override, RuntimeParagraphSpacingOverride);
}

#[derive(Clone)]
#[doc(hidden)]
pub struct InheritedScopeValue {
    pub(crate) value: Rc<dyn Any>,
}

impl InheritedScopeValue {
    /// The payload's own type. Dispatches through the `dyn Any` vtable;
    /// calling `type_id` on the `Rc` itself would name `Rc<dyn Any>`.
    pub(super) fn value_type(&self) -> TypeId {
        (*self.value).type_id()
    }

    pub(super) fn new<T: Any>(value: T) -> Self {
        Self {
            value: Rc::new(value),
        }
    }

    pub(super) fn from_shared<T: Any>(value: Rc<T>) -> Self {
        Self { value }
    }
}
