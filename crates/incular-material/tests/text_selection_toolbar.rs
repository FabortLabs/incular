//! Flutter's Material `TextSelectionToolbar` (Android): a filled, elevated
//! pill of buttons placed by `TextSelectionToolbarLayoutDelegate`, above the
//! anchor when it fits under the top padding and below it otherwise. The
//! toolbar must paint its surface even when no `Theme` is above it, as in an
//! app without a Material theme scope.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_material::TextSelectionToolbar;
use incular_rendering::{Brush, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{Directionality, SizedBox, Widget};

const SCREEN: Size = Size {
    width: 400.,
    height: 300.,
};

fn toolbar(anchor_above: Offset, anchor_below: Offset) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        TextSelectionToolbar::new(
            anchor_above,
            anchor_below,
            vec![
                SizedBox::from_dimensions(Some(60.), Some(30.), None::<Widget>).into(),
                SizedBox::from_dimensions(Some(60.), Some(30.), None::<Widget>).into(),
            ],
        ),
    )
    .into()
}

/// Every solid rectangle, in global coordinates, with its colour. Only
/// translations are tracked, which is all the toolbar's layout uses.
fn solid_rects(list: &incular_rendering::DisplayList) -> Vec<(Rect, Color)> {
    let mut stack = vec![Transform::IDENTITY];
    let mut rects = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let current = *stack.last().expect("transform stack");
                stack.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                stack.pop();
            }
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } => {
                let current = *stack.last().expect("transform stack");
                rects.push((current.transform_rect_bbox(rrect.rect), *color));
            }
            _ => {}
        }
    }
    rects
}

fn painted_toolbar(anchor_above: Offset, anchor_below: Offset) -> Vec<(Rect, Color)> {
    let mut runtime = Runtime::new(toolbar(anchor_above, anchor_below)).expect("runtime");
    let (list, _) = runtime
        .run_frame(Constraints::tight(SCREEN))
        .expect("frame");
    solid_rects(&list)
        .into_iter()
        .filter(|(rect, _)| rect.size.height == 44.)
        .collect()
}

#[test]
fn toolbar_paints_a_filled_surface_without_a_theme() {
    let rects = painted_toolbar(Offset::new(200., 200.), Offset::new(200., 220.));
    let (_, color) = rects.first().copied().expect("toolbar surface");
    // Flutter's default light surface is Colors.white.
    assert_eq!(color, Color::WHITE);
}

#[test]
fn toolbar_sits_above_the_anchor_when_it_fits() {
    let rects = painted_toolbar(Offset::new(200., 200.), Offset::new(200., 220.));
    let (rect, _) = rects[0];
    // The bottom sits _kToolbarContentDistance (8) above the anchor.
    assert_eq!(rect.origin.y + rect.size.height, 192.);
    assert_eq!(rect.origin.y, 148.);
}

#[test]
fn toolbar_flips_below_the_anchor_without_room_above() {
    // 30 - 2 * 8 - 8 (screen padding) = 6 pixels above: too little for 44.
    let rects = painted_toolbar(Offset::new(200., 30.), Offset::new(200., 60.));
    let (rect, _) = rects[0];
    // The top sits kToolbarContentDistanceBelow (20) below the anchor.
    assert_eq!(rect.origin.y, 80.);
}
