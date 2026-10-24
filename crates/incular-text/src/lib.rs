//! Renderer-independent text primitives and cached logical text layout.
//!
//! The public surface is split into small modules so consumers can use the
//! text model (`spans`), editor state (`editing`), and shaping engine
//! independently. Existing font/layout exports remain available at the crate
//! root for backwards compatibility.
mod boundary;
mod editing;
mod spans;
mod style;

pub use boundary::{TextBoundaryKind, set_word_segmenter};
pub use editing::{
    ComposingRange, EditableText, SelectionChangedCause, TextAffinity, TextEditingController,
    TextEditingDelta, TextEditingSubscription, TextEditingValue, TextField, TextRange,
    TextSelection,
};
pub use spans::{
    InlineSpan, RichText, Text, TextSpan, TextSpanVisitor, WidgetSpan, WidgetSpanAlignment,
};
pub use style::{
    FontFamily, FontFeature, FontStyle, FontVariation, FontWeight, IconData, LineHeight,
    StrutStyle, TextAlign, TextBaseline, TextDecoration, TextDecorationStyle, TextHeightBehavior,
    TextLeadingDistribution, TextOverflow, TextScaler, TextScalerKind, TextShadow, TextStyle,
    TextWidthBasis,
};

mod engine;
pub use engine::{
    FontId, FontRunDebug, TextCaretPosition, TextClusterSpan, TextDiagnostics, TextDocumentLayout,
    TextEngine, TextLayout, TextLayoutOptions, TextLine, TextMetrics, register_application_font,
    set_missing_characters_listener, take_missing_characters,
};
mod geometry;
pub use geometry::{TextGeometry, TextLineRef, TextLines};
