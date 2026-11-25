//! Native Windows wgpu readback for both pinned ClipRRect raster modes.
#![cfg(target_os = "windows")]
#[path = "support/clip_rrect_raster.rs"]
mod fixture;
use std::sync::Arc;

use fixture::{BLACK, WHITE, capture, with_renderer};
use incular_config::Clip;
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{Canvas, FillRule, PathBuilder, RRect};
use incular_wgpu::{FrameOutcome, WgpuRenderer};

fn raw_clip(renderer: &mut WgpuRenderer, factor: f32, path_clip: Option<bool>) -> [[u8; 4]; 4] {
    let mut canvas = Canvas::default();
    canvas.save_transform(Transform::translation(Offset::new(4., 4.)));
    canvas.save_transform(Transform::scale(factor));
    let size = 40. / factor;
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(size, size));
    match path_clip {
        Some(false) => canvas.save_clip_rrect(RRect::uniform(rect, 12. / factor)),
        Some(true) => {
            let edge = 12. / factor;
            let mut path = PathBuilder::default();
            path.move_to(Offset::new(edge, 0.))
                .line_to(Offset::new(size, 0.))
                .line_to(Offset::new(size, size))
                .line_to(Offset::new(0., size))
                .line_to(Offset::new(0., edge))
                .close();
            canvas.save_clip_path(Arc::new(path.build()), FillRule::NonZero);
        }
        None => {}
    }
    canvas.rect(rect, Color::WHITE);
    if path_clip.is_some() {
        canvas.restore();
    }
    canvas.restore();
    canvas.restore();
    renderer.request_capture();
    assert!(matches!(
        renderer.render(&canvas.finish(), 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = renderer.take_capture().unwrap().unwrap();
    [(4, 4), (16, 16), (34, 16), (45, 16)].map(|(x, y)| {
        let index = ((y * frame.width + x) * 4) as usize;
        frame.rgba8[index..index + 4].try_into().unwrap()
    })
}
#[test]
fn flutter_350_clip_rrect_modes_distinguish_aa_and_saved_compositing() {
    with_renderer(|renderer| {
        let none = capture(renderer, Some(Clip::None), false);
        assert_eq!((none.corner, none.center), (WHITE, WHITE), "clip none");
        let hard = capture(renderer, Some(Clip::HardEdge), false);
        let default_clip = capture(renderer, None, false);
        let ordinary_overlap = capture(renderer, None, true);
        let saved_overlap = capture(renderer, Some(Clip::AntiAliasWithSaveLayer), true);
        for samples in [&hard, &default_clip, &ordinary_overlap, &saved_overlap] {
            assert_eq!((samples.corner, samples.center), (BLACK, WHITE));
        }
        assert_eq!(hard.partial, 0, "hardEdge has no partially covered pixels");
        assert!(
            default_clip.partial > 0
                && ordinary_overlap.partial > 0
                && ordinary_overlap.red_fringe > 0
                && saved_overlap.partial > 0
                && saved_overlap.red_fringe == 0,
            "default should smooth edges and permit red fringe when painting twice; \
         saveLayer should smooth edges without that fringe. Got default={default_clip:?}, \
         ordinary_overlap={ordinary_overlap:?}, saved_overlap={saved_overlap:?}"
        );

        let expected = [BLACK, WHITE, WHITE, BLACK];
        let unscaled_rrect = raw_clip(renderer, 1., Some(false));
        let scaled_rrect = raw_clip(renderer, 2., Some(false));
        let unscaled_path = raw_clip(renderer, 1., Some(true));
        let scaled_path = raw_clip(renderer, 2., Some(true));
        assert_eq!(raw_clip(renderer, 2., None), [WHITE, WHITE, WHITE, BLACK]);
        assert_eq!(
            [unscaled_rrect, scaled_rrect, unscaled_path, scaled_path],
            [expected; 4],
            "the scissor follows the full transform for rounded and path clips"
        );
    });
}
