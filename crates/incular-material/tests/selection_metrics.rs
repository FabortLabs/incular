//! Flutter's Material 3 selection-control geometry: `_SwitchConfigM3` (a
//! 52x32 track, a 16 px thumb off and 24 px on, centered half the track
//! height from each end) and `_SwitchDefaultsM3.trackOutlineWidth`. The
//! thumb's slide is a compositor translation, outside the paint stream.

use incular_config::Constraints;
use incular_core::{Rect, Size, Transform};
use incular_material::Switch;
use incular_rendering::PaintCommand;
use incular_widgets::internal::WidgetTree;

fn painted_rects(switch: Switch) -> (Vec<Rect>, Vec<f32>) {
    let mut tree = WidgetTree::new();
    tree.mount(switch.into()).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    let painted = tree.paint();
    let mut rects = Vec::new();
    let mut borders = Vec::new();
    let mut transforms = vec![Transform::IDENTITY];
    let place = |transforms: &[Transform], rect: Rect| {
        let transform = *transforms.last().expect("root transform");
        Rect::from_origin_size(transform.transform_point(rect.origin), rect.size)
    };
    for command in painted.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let current = *transforms.last().expect("root transform");
                transforms.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::RRect { rrect, .. } => rects.push(place(&transforms, rrect.rect)),
            PaintCommand::Border { rrect, border } => {
                rects.push(place(&transforms, rrect.rect));
                borders.push(border.width);
            }
            _ => {}
        }
    }
    (rects, borders)
}

fn find(rects: &[Rect], width: f32, height: f32) -> Rect {
    *rects
        .iter()
        .find(|rect| rect.size == Size::new(width, height))
        .unwrap_or_else(|| panic!("no {width}x{height} rect in {rects:?}"))
}

#[test]
fn switch_track_and_thumb_follow_material_3() {
    let (rects, borders) = painted_rects(Switch::new(false));
    let track = find(&rects, 52., 32.);
    let thumb = find(&rects, 16., 16.);
    // Off: the thumb's center is 16 from the track's left, 8 + 16 / 2.
    assert_eq!(thumb.origin.x - track.origin.x, 8.);
    assert_eq!(thumb.origin.y - track.origin.y, 8.);
    assert!(borders.contains(&2.), "track outline width 2: {borders:?}");

    // On: the thumb grows to 24 px, inset 4 from the track's left end; the
    // compositor then slides it along the 52 - 32 = 20 px travel, which
    // leaves its center 16 from the right end.
    let (rects, _) = painted_rects(Switch::new(true));
    let track = find(&rects, 52., 32.);
    let thumb = find(&rects, 24., 24.);
    assert_eq!(thumb.origin.x - track.origin.x, 4.);
    assert_eq!(thumb.origin.y - track.origin.y, 4.);
}
