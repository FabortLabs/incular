//! Renderer-independent text primitives and cached logical text layout.
//!
//! The public surface is split into small modules so consumers can use the
//! text model (`spans`), editor state (`editing`), and shaping engine
//! independently. Existing font/layout exports remain available at the crate
//! root for backwards compatibility.
mod boundary;
mod editing;
mod flutter_editing_delta;
mod font_loader;
mod spans;
mod style;
mod text_painter;
pub use text_painter::{
    ParagraphCaretContext,
    Hyphens, PlaceholderDimensions, TextPainter, TextPainterBox, TextPainterError, TextPainterLine,
    TextPainterPaint, TextPainterPosition, TextPainterPositionError,
};
#[cfg(all(target_os = "windows", not(feature = "bundled-dictionaries")))]
mod windows_segmentation;

pub use boundary::{
    LineBoundary, TextBoundaryKind, accessibility_word_cursor, is_layout_whitespace, set_word_segmenter, word_boundary_at,
};
pub use editing::{
    ComposingRange, EditableText, FlutterTextSelection, FocusedSelectionLease, InvalidTextDeltaRange,
    SelectionChangedCause, TextAffinity, TextEditingController, TextEditingDelta,
    TextEditingSubscription, TextEditingValue, TextField, TextRange, TextSelection,
    byte_offset_at_utf16_boundary,
};
pub use flutter_editing_delta::{FlutterTextDeltaError, FlutterTextEdit, FlutterTextEditingDelta};
pub use font_loader::{FontLoader, FontLoaderError};
pub use spans::{
    CombinedInlineSpanSemantics, InlineSpan, InlineSpanComparison, InlineSpanRef,
    InlineSpanSemanticsInformation, InlineSpanStringAttribute, InvalidUtf16Span, PlainTextOptions,
    RichText,
    StyledTextRun, Text, TextSpan, TextSpanMouse, TextSpanVisitor, WidgetSpan,
    WidgetSpanAlignment, combine_inline_span_semantics,
};
pub use style::{
    FontFamily, FontFeature, FontStyle, FontVariation, FontWeight, IconData, LineHeight,
    StrutStyle, TextAlign, TextBaseline, TextDecoration, TextDecorationStyle, TextHeightBehavior,
    TextLeadingDistribution, TextMetricAdjustment, TextOverflow, TextScaleBoundsError, TextScaler,
    TextScalerKind, TextShadow, TextStyle, TextStyleApply, TextWidthBasis, lerp_font_variations,
};

mod engine;
pub use engine::{
    FontId, FontRunDebug, TextBackgroundRect, TextCaretPosition, TextClusterSpan, TextColorRun,
    TextDiagnostics, TextDocumentLayout, TextEngine, TextIntrinsicWidths, TextLayout, TextLayoutOptions, TextLine,
    TextMetrics, TextPlaceholder, TextPlaceholderBox, TextStrut, register_application_font,
    register_application_font_as, set_missing_characters_listener, take_missing_characters,
};
mod geometry;
pub use geometry::{TextGeometry, TextLineRef, TextLines};
