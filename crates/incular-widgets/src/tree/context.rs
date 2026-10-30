//! Build-time inherited environment access.
//!
//! This module owns borrowed build context views and inherited descriptor values; retained dependency tracking remains owned by `WidgetTree`.

use super::*;
use incular_config::{
    Brightness, DisplayFeature, InputCapabilities, Locale, LocaleResolver, RuntimeEnvironment,
    TextDirection,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeViewport(Size);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeScaleFactor(f64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeBrightness(Brightness);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeTextScale(f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeSafeInsets(EdgeInsets);
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
}

/// Flutter's `MediaQueryData.removeDisplayFeatures`, for the subtree of a
/// sub-screen: the given features and insets replace the window's.
pub(crate) fn sub_screen_scope(
    features: Vec<DisplayFeature>,
    safe_insets: EdgeInsets,
    view_insets: EdgeInsets,
    child: Widget,
) -> Widget {
    Widget::environment_scope(
        RuntimeDisplayFeatures(features),
        Widget::environment_scope(
            RuntimeSafeInsets(safe_insets),
            Widget::environment_scope(RuntimeViewInsets(view_insets), child),
        ),
    )
}

pub(super) fn install_runtime_environment(
    context: &DependencyContext,
    environment: &RuntimeEnvironment,
) {
    // Flutter reads `disableAnimations` from the platform's accessibility
    // features; here it follows the window's reduced-motion preference.
    incular_animation::set_disable_animations(environment.reduced_motion);
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

    replace_if_changed!(viewport, RuntimeViewport);
    replace_if_changed!(scale_factor, RuntimeScaleFactor);
    replace_if_changed!(brightness, RuntimeBrightness);
    replace_if_changed!(text_scale, RuntimeTextScale);
    replace_if_changed!(safe_insets, RuntimeSafeInsets);
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
    pub(crate) type_id: TypeId,
    pub(crate) value: Rc<dyn Any>,
}

impl InheritedScopeValue {
    pub(super) fn new<T: Any>(value: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            value: Rc::new(value),
        }
    }

    pub(super) fn from_shared<T: Any>(value: Rc<T>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            value,
        }
    }
}
