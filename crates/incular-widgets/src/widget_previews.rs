//! Programmatic, design-system-neutral widget preview composition.
//!
//! This module configures directly supplied widgets. It neither scans source
//! for preview annotations nor starts a preview server.

use std::rc::Rc;

use incular_config::{Brightness, Locale, LocaleResolver, LocalizationCatalog};
use incular_core::Size;

use crate::{BuildContext, Builder, Directionality, Widget};

/// Wraps an existing preview widget in additional scaffolding.
pub type WidgetWrapper = Rc<dyn Fn(Widget) -> Widget>;

/// Creates the preview's theme on demand.
pub type PreviewTheme = Rc<dyn Fn() -> Rc<dyn PreviewThemeData>>;

/// Creates localization configuration for the preview on demand.
pub type PreviewLocalizations = Rc<dyn Fn() -> PreviewLocalizationsData>;

/// Resolution override for the platform's ordered locale preferences.
pub type PreviewLocaleListResolution = Rc<dyn Fn(&[Locale], &[Locale]) -> Option<Locale>>;

/// Resolution override for the first platform locale, or no preference.
pub type PreviewLocaleResolution = Rc<dyn Fn(Option<&Locale>, &[Locale]) -> Option<Locale>>;

/// Locale, resources and locale-resolution callbacks for a preview.
///
/// Incular's neutral localization service exposes an application catalog
/// rather than a Dart delegate list, while retaining the preview's locale
/// override, supported locales, and resolution order.
#[derive(Clone)]
pub struct PreviewLocalizationsData {
    pub locale: Option<Locale>,
    pub supported_locales: Vec<Locale>,
    pub catalog: Option<Rc<dyn LocalizationCatalog>>,
    pub locale_list_resolution: Option<PreviewLocaleListResolution>,
    pub locale_resolution: Option<PreviewLocaleResolution>,
}

impl Default for PreviewLocalizationsData {
    fn default() -> Self {
        Self {
            locale: None,
            supported_locales: vec!["en-US".parse().expect("valid default preview locale")],
            catalog: None,
            locale_list_resolution: None,
            locale_resolution: None,
        }
    }
}

impl PreviewLocalizationsData {
    /// Resolves the explicit locale or the ordered platform preferences.
    /// A null result from either override falls through to the next stage.
    #[must_use]
    pub fn resolve(&self, platform: &[Locale]) -> Option<Locale> {
        let preferred = self.locale.as_ref().map_or(platform, std::slice::from_ref);
        let supported = &self.supported_locales;
        self.locale_list_resolution
            .as_ref()
            .and_then(|resolve| resolve(preferred, supported))
            .or_else(|| {
                self.locale_resolution
                    .as_ref()
                    .and_then(|resolve| resolve(preferred.first(), supported))
            })
            .or_else(|| LocaleResolver::resolve_basic(preferred, supported))
    }
}

struct EmptyPreviewCatalog(Vec<Locale>);

impl LocalizationCatalog for EmptyPreviewCatalog {
    fn supported_locales(&self) -> &[Locale] {
        &self.0
    }

    fn message(&self, _: &Locale, _: &str) -> Option<&str> {
        None
    }
}

/// A context-aware theme for a preview. Implementations may use any
/// application environment or design system.
pub trait PreviewThemeData {
    /// Applies this theme to the already composed child.
    fn apply(&self, context: &BuildContext<'_>, child: Widget) -> Widget;
}

impl<F> PreviewThemeData for F
where
    F: for<'a> Fn(&BuildContext<'a>, Widget) -> Widget,
{
    fn apply(&self, context: &BuildContext<'_>, child: Widget) -> Widget {
        self(context, child)
    }
}

/// Composes themes with the first supplied theme outermost.
#[derive(Clone, Default)]
pub struct MultiPreviewThemeData {
    themes: Vec<Rc<dyn PreviewThemeData>>,
}

impl MultiPreviewThemeData {
    #[must_use]
    pub fn new(themes: impl IntoIterator<Item = Rc<dyn PreviewThemeData>>) -> Self {
        Self {
            themes: themes.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn themes(&self) -> &[Rc<dyn PreviewThemeData>] {
        &self.themes
    }
}

impl PreviewThemeData for MultiPreviewThemeData {
    fn apply(&self, context: &BuildContext<'_>, child: Widget) -> Widget {
        self.themes
            .iter()
            .rev()
            .fold(child, |child, theme| theme.apply(context, child))
    }
}

/// Mutable configuration for a directly constructed widget preview.
#[derive(Clone, Default)]
pub struct PreviewBuilder {
    /// Group label. If omitted, the built preview uses `"Default"`.
    pub group: Option<String>,
    /// Optional per-preview label.
    pub name: Option<String>,
    /// Optional simulated size for the embedding host.
    pub size: Option<Size>,
    /// Optional text scale for the embedding host.
    pub text_scale_factor: Option<f32>,
    /// Optional initial brightness for the embedding host.
    pub brightness: Option<Brightness>,
    /// Optional factory for context-aware theme data.
    pub theme: Option<PreviewTheme>,
    /// Optional factory for the preview's locale and resources.
    pub localizations: Option<PreviewLocalizations>,
    wrappers: Vec<WidgetWrapper>,
}

impl PreviewBuilder {
    /// Starts with no configured options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the complete wrapper chain, or removes it with `None`.
    pub fn set_wrapper(&mut self, wrapper: Option<WidgetWrapper>) {
        self.wrappers.clear();
        self.wrappers.extend(wrapper);
    }

    /// Adds an outer wrapper around the result of all previous wrappers.
    pub fn add_wrapper(&mut self, wrapper: impl Fn(Widget) -> Widget + 'static) {
        self.wrappers.push(Rc::new(wrapper));
    }

    /// Creates an independent snapshot. Further builder changes do not change
    /// this preview or its wrapper order.
    #[must_use]
    pub fn build(&self) -> Preview {
        Preview(self.clone())
    }
}

/// Runtime metadata and composition for a widget preview.
#[derive(Clone, Default)]
pub struct Preview(PreviewBuilder);

impl Preview {
    #[must_use]
    pub fn group(&self) -> &str {
        self.0.group.as_deref().unwrap_or("Default")
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    #[must_use]
    pub fn size(&self) -> Option<Size> {
        self.0.size
    }

    #[must_use]
    pub fn text_scale_factor(&self) -> Option<f32> {
        self.0.text_scale_factor
    }

    #[must_use]
    pub fn brightness(&self) -> Option<Brightness> {
        self.0.brightness
    }

    /// Clones this preview's resolved metadata and wrapper chain into a
    /// builder that may be modified without affecting the original preview.
    #[must_use]
    pub fn to_builder(&self) -> PreviewBuilder {
        let mut builder = self.0.clone();
        if builder.group.is_none() {
            builder.group = Some(String::from("Default"));
        }
        builder
    }

    /// Applies all configured wrappers, in order from innermost to outermost.
    #[must_use]
    pub fn wrap(&self, child: impl Into<Widget>) -> Widget {
        self.0
            .wrappers
            .iter()
            .fold(child.into(), |child, wrapper| wrapper(child))
    }

    /// Builds the optional theme and applies it once using the supplied
    /// context. Without a theme the child is unchanged.
    #[must_use]
    pub fn apply_theme(&self, context: &BuildContext<'_>, child: impl Into<Widget>) -> Widget {
        let child = child.into();
        match &self.0.theme {
            Some(theme) => theme().apply(context, child),
            None => child,
        }
    }

    /// Installs a retained localization scope for the preview subtree.
    /// The factory runs once when composing this widget; when inherited
    /// platform locales change, only the scope and its consumers update.
    #[must_use]
    pub fn apply_localizations(&self, child: impl Into<Widget>) -> Widget {
        let child = child.into();
        let Some(factory) = &self.0.localizations else {
            return child;
        };
        let data = factory();
        let catalog: Rc<dyn LocalizationCatalog> = data
            .catalog
            .clone()
            .unwrap_or_else(|| Rc::new(EmptyPreviewCatalog(data.supported_locales.clone())));
        Builder::new(move |context| {
            let platform = if data.locale.is_none() {
                context.locales()
            } else {
                Vec::new()
            };
            let Some(locale) = data.resolve(&platform) else {
                return child.clone();
            };
            let direction = LocaleResolver::text_direction(&locale);
            Widget::environment_scope(
                (locale, catalog.clone()),
                Directionality::new(direction, child.clone()).into(),
            )
        })
        .into()
    }
}
