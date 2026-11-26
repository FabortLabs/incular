//! Typography values shared by plain and rich text.

mod fonts;
mod icons;
mod text;
mod text_style;

pub use fonts::{FontFamily, FontFeature, FontVariation, FontWeight, lerp_font_variations};
pub use icons::IconData;
pub use text::{
    FontStyle, LineHeight, StrutStyle, TextAlign, TextBaseline, TextDecoration,
    TextDecorationStyle, TextHeightBehavior, TextLeadingDistribution, TextOverflow, TextScaleBoundsError, TextScaler,
    TextScalerKind, TextShadow, TextWidthBasis,
};
pub use text_style::{TextMetricAdjustment, TextStyle, TextStyleApply};
