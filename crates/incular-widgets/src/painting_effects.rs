//! Painting, image, masking, clipping, and compositor effect widgets.

mod box_painter;
mod custom_paint;

pub use custom_paint::PainterDelegate;
mod decoration;
mod geometry;
mod images;
mod layers;
mod shape_borders;
mod shape_decoration;

pub use box_painter::DecorationPosition;
pub(crate) use box_painter::{ImagePaint, decorated, painted_image};
pub use custom_paint::{CustomPainter, CustomPainterSemantics, SemanticsBuilder};
pub use decoration::{
    BlurStyle, Border, BorderDirectional, BorderSide, BorderStyle, BoxBorder, BoxDecoration,
    BoxShadow, BoxShape, DecorationImage, TileMode,
};
pub use geometry::{BorderRadius, BorderRadiusDirectional, Radius};
pub use images::{ImageFiltered, ImageIcon, RawImage};
pub use layers::{ClipRSuperellipse, GridPaper, PhysicalModel, PhysicalShape, SnapshotWidget};
pub use shape_borders::{
    BeveledRectangleBorder, CircleBorder, ContinuousRectangleBorder, LinearBorder,
    LinearBorderEdge, OvalBorder, RoundedRectangleBorder, RoundedSuperellipseBorder, ShapeBorder,
    StadiumBorder, StarBorder, StarBorderError,
};
pub use shape_decoration::{DecoratedShapeBox, ShapeBorderClipper, ShapeDecoration};
