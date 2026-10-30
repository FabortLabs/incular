//! Painting, image, masking, clipping, and compositor effect widgets.

mod box_painter;
mod custom_paint;
mod decoration;
mod geometry;
mod images;
mod layers;
mod shape_borders;
mod shape_decoration;

pub use box_painter::DecorationPosition;
pub(crate) use box_painter::decorated;
pub use custom_paint::CustomPainter;
pub use decoration::{
    BlurStyle, Border, BorderDirectional, BorderSide, BorderStyle, BoxBorder, BoxDecoration,
    BoxShadow, BoxShape, DecorationImage, TileMode,
};
pub use geometry::{BorderRadius, BorderRadiusDirectional, Radius};
pub use images::{ImageFiltered, ImageIcon, RawImage};
pub use layers::{ClipRSuperellipse, GridPaper, PhysicalModel, PhysicalShape, SnapshotWidget};
pub use shape_borders::{
    RoundedRectangleBorder, RoundedSuperellipseBorder, ShapeBorder, StadiumBorder, StarBorder,
};
pub use shape_decoration::{DecoratedShapeBox, ShapeDecoration};
