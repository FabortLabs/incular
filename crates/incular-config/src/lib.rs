//! Renderer-independent configuration values shared by Incular layers.
//!
//! The types in this crate describe policy and bounds only. Layout algorithms
//! live in `incular-layout`; widgets, runtime, and platform adapters can use
//! these values without depending on each other.

mod alignment;
mod alignment_geometry;
mod constraints;
mod defaults;
mod environment;
mod insets;
mod insets_geometry;
mod localization;
mod transient;

pub use alignment::{
    Alignment, AlignmentDirectional, Axis, AxisDirection, Clip, CrossAxisAlignment, FlexFit,
    FractionalOffset, MainAxisAlignment, MainAxisSize, StackFit, VerticalDirection, WrapAlignment,
    WrapCrossAlignment,
};
pub use alignment_geometry::{AlignmentGeometry, AlignmentResolutionError};
pub use constraints::{ConstraintError, Constraints, ViewConstraints};
pub use defaults::{ApplicationDefaults, TransparencyMode, WidgetDefaults, WindowSizePolicy};
pub use environment::{
    Brightness, ContentSensitivity, DeviceGestureSettings, DisplayFeature, DisplayFeatureState,
    DisplayFeatureType, InputCapabilities, MediaQueryEdges, NavigationMode, RuntimeEnvironment,
    TargetPlatform,
};
pub use icu_locale::Locale;
pub use incular_core::TextDirection;
pub use insets::{EdgeInsets, EdgeInsetsDirectional};
pub use insets_geometry::{EdgeInsetsGeometry, MissingInsetsDirection};
pub use localization::{
    LocaleResolver, LocalizationCatalog, LocalizationError, LocalizedMessage, PluralCategory,
    PluralForms,
};
pub use transient::{TransientPresentation, TransientRole};

// Preserve the equality of IEEE positive and negative zero in value keys.
fn hash_layout_floats<const N: usize>(values: [f32; N], state: &mut impl std::hash::Hasher) {
    let bits = values.map(|value| if value == 0. { 0 } else { value.to_bits() });
    std::hash::Hash::hash(&bits, state);
}
