//! Painting, image, masking, clipping, and compositor effect widgets.

mod box_painter;
mod box_border_stack;
pub use box_border_stack::{BoxBorderLayer, BoxBorderStack};
mod custom_paint;
mod custom_box_border;

pub use custom_paint::PainterDelegate;
pub use custom_box_border::{BoxBorderValue, CustomBoxBorder, CustomBoxBorderHandle};
mod decoration;
mod geometry;
mod images;
mod layers;
mod rounded_circle_outline;
pub use rounded_circle_outline::RoundedCircleOutline;
mod shape_borders;
mod shape_decoration;
mod table_border;
pub use table_border::TableBorder;

pub use box_painter::DecorationPosition;
pub(crate) use box_painter::{ImagePaint, decorated, paint_image};
pub use custom_paint::{CustomPainter, CustomPainterSemantics, SemanticsBuilder};
pub use decoration::{
    BlurStyle, Border, BorderDirectional, BorderSide, BorderStyle, BoxBorder, BoxDecoration,
    BoxLinearGradient, BoxRadialGradient, BoxShadow, BoxShape, BoxSweepGradient, DecorationImage,
    TileMode,
};
pub use geometry::{
    BorderRadius, BorderRadiusDirectional, BorderRadiusGeometry, EllipticalRoundedRect, Radius,
};
pub(crate) use images::RasterPaintSources;
pub use images::{ImageFiltered, ImageIcon, RawImage};
pub use layers::{ClipRSuperellipse, GridPaper, PhysicalModel, PhysicalShape, SnapshotWidget};
pub use shape_borders::{
    BeveledRectangleBorder, BorderComposition, CircleBorder, CompoundBorder,
    ContinuousRectangleBorder, LinearBorder, LinearBorderEdge, OvalBorder, RoundedRectangleBorder,
    RoundedSuperellipseBorder, ShapeBorder, StadiumBorder, StarBorder, StarBorderError, StarBorderUpdate,
};
pub use shape_decoration::{AlignedShapeGradient, DecoratedShapeBox, ShapeBorderClipper, ShapeDecoration};


// IEEE positive and negative zero compare equal in painting value types.
fn hash_paint_float(value: f32, state: &mut impl std::hash::Hasher) {
    let bits = if value == 0.0 { 0 } else { value.to_bits() };
    std::hash::Hash::hash(&bits, state);
}
