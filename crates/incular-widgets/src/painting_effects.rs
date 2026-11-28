//! Painting, image, masking, clipping, and compositor effect widgets.

mod box_painter;
mod box_border;
mod box_border_stack;
mod box_shadow_values;
pub use box_border::BoxBorderOutline;
pub use box_border_stack::{BoxBorderLayer, BoxBorderStack};
mod custom_paint;
mod custom_box_border;

pub use custom_paint::PainterDelegate;
pub use custom_box_border::{BoxBorderValue, CustomBoxBorder, CustomBoxBorderHandle};
mod decoration;
mod geometry;
mod images;
mod layers;
mod notched_shape;
pub use notched_shape::{CircularNotchedRectangle, NotchedShape};
mod rounded_circle_outline;
pub use rounded_circle_outline::RoundedCircleOutline;
mod shape_borders;
mod shape_decoration;
mod table_border;
mod value_hash;
pub use table_border::TableBorder;

pub use box_painter::DecorationPosition;
pub(crate) use box_painter::{ImagePaint, decorated, paint_image};
pub use custom_paint::{CustomPainter, CustomPainterSemantics, SemanticsBuilder};
pub use decoration::{
    BlurStyle, Border, BorderDirectional, BorderSide, BorderStyle, BoxBorder, BoxDecoration,
    BoxLinearGradient, BoxRadialGradient, BoxShadow, BoxShape, BoxSweepGradient, DecorationImage, DecorationImageErrorHandler, DecorationImageSource,
    TileMode,
};
pub use geometry::{
    BorderRadius, BorderRadiusDirectional, BorderRadiusGeometry, BorderRadiusResolutionError,
    EllipticalRoundedRect, Radius,
};
pub(crate) use images::RasterPaintSources;
pub use images::{ImageFiltered, ImageIcon, RawImage};
pub use layers::{ClipRSuperellipse, GridPaper, PhysicalModel, PhysicalShape, SnapshotWidget};
pub use shape_borders::{
    BeveledRectangleBorder, BorderComposition, CircleBorder, CompoundBorder,
    ContinuousRectangleBorder, LinearBorder, LinearBorderEdge, OvalBorder, RoundedRectangleBorder,
    RoundedSuperellipseBorder, ShapeBorder, StadiumBorder, StarBorder, StarBorderError, StarBorderUpdate,
    lerp_shape_border,
};
pub use shape_decoration::{AlignedShapeGradient, DecoratedShapeBox, ShapeBorderClipper, ShapeDecoration};


// Retain the input allocation when static nullable interpolation was
// called with two references to the same value.
fn lerp_borrowed<'a, T: Copy>(
    begin: Option<&'a T>,
    end: Option<&'a T>,
    t: f32,
    interpolate: impl FnOnce(Option<T>, Option<T>, f32) -> Option<T>,
) -> Option<std::borrow::Cow<'a, T>> {
    if let (Some(a), Some(b)) = (begin, end)
        && std::ptr::eq(a, b)
    {
        return Some(std::borrow::Cow::Borrowed(a));
    }
    interpolate(begin.copied(), end.copied(), t).map(std::borrow::Cow::Owned)
}

// IEEE positive and negative zero compare equal in painting value types.
fn hash_paint_float(value: f32, state: &mut impl std::hash::Hasher) {
    let bits = if value == 0.0 { 0 } else { value.to_bits() };
    std::hash::Hash::hash(&bits, state);
}
