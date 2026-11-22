//! Material text selection handles: Flutter's `MaterialTextSelectionHandleControls`
//! (`material/text_selection.dart` `buildHandle`) and its
//! `_TextSelectionHandlePainter`, a circle with a square corner.

use super::text_selection_toolbar::toolbar_theme;
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{Canvas, CornerRadii, DisplayList, FillRule, Path};
use incular_widgets::{
    BuildContext, CustomPaint, Positioned, SELECTION_HANDLE_SIZE, SelectionHandleType, Stack,
    StackFit, TextSelectionHandle, TextSelectionHandles, TextSelectionHandlesBuilder, Widget,
};
use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_4},
    rc::Rc,
    sync::Arc,
};

/// Material's `TextSelectionHandleControls.buildHandle`: a teardrop in the
/// theme's primary colour, at each handle's anchor. Flutter's
/// `TextSelectionTheme.selectionHandleColor` is not modelled yet.
#[must_use]
pub fn default_text_selection_handles_builder() -> TextSelectionHandlesBuilder {
    Rc::new(
        |context: &BuildContext<'_>, handles: &TextSelectionHandles| {
            let color = toolbar_theme(context).core().color_scheme.primary;
            let mut children = vec![handle_widget(&handles.start, color)];
            if let Some(end) = &handles.end {
                children.push(handle_widget(end, color));
            }
            Stack::new(children)
                .fit(StackFit::Passthrough)
                .clip_behavior(incular_config::Clip::None)
                .into()
        },
    )
}

/// Places one handle with its box at `getHandleAnchor`'s offset.
fn handle_widget(handle: &TextSelectionHandle, color: Color) -> Widget {
    let rect = handle.rect();
    Positioned::builder()
        .left(rect.origin.x)
        .top(rect.origin.y)
        .width(SELECTION_HANDLE_SIZE)
        .height(SELECTION_HANDLE_SIZE)
        .child(CustomPaint::new(
            Size::new(SELECTION_HANDLE_SIZE, SELECTION_HANDLE_SIZE),
            handle_display_list(handle.handle_type, color),
        ))
        .build()
        .into()
}

/// Flutter's `Transform.rotate` for each handle, so the square corner points
/// up-right for a start handle, up-left for an end handle and up for a
/// collapsed one.
fn handle_angle(handle_type: SelectionHandleType) -> f32 {
    match handle_type {
        SelectionHandleType::Left => FRAC_PI_2,
        SelectionHandleType::Right => 0.0,
        SelectionHandleType::Collapsed => FRAC_PI_4,
    }
}

/// `_TextSelectionHandlePainter.paint`: the circle of diameter
/// [`SELECTION_HANDLE_SIZE`] plus its top-left quadrant as a square, drawn
/// as one rounded rectangle whose top-left radius is zero.
fn handle_display_list(handle_type: SelectionHandleType, color: Color) -> DisplayList {
    let size = Size::new(SELECTION_HANDLE_SIZE, SELECTION_HANDLE_SIZE);
    let radius = SELECTION_HANDLE_SIZE / 2.0;
    let mut builder = Path::builder();
    builder.rounded_rect(
        Rect::from_origin_size(Offset::new(0.0, 0.0), size),
        CornerRadii {
            top_left: 0.0,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        },
    );
    let path = Arc::new(builder.build());
    // Rotate about the centre: translate it to the origin, rotate, and back.
    let centre = Offset::new(radius, radius);
    let transform = Transform::translation(centre)
        .then(Transform::rotation(handle_angle(handle_type)))
        .then(Transform::translation(Offset::new(-radius, -radius)));
    let mut canvas = Canvas::default();
    canvas.save_transform(transform);
    canvas.fill_path(path, color, FillRule::NonZero);
    canvas.restore();
    canvas.finish()
}
