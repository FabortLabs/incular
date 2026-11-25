//! Native Windows wgpu readback for both pinned ClipRRect raster modes.
#![cfg(target_os = "windows")]
#[path = "support/clip_rrect_raster.rs"]
mod fixture;
use std::sync::Arc;

use fixture::{BLACK, WHITE, capture, with_renderer};
use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{Canvas, ClipCoverage, DisplayList, FillRule, PathBuilder, RRect};
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer};
use incular_widgets::{ClipOval, Positioned, Stack, Widget};

#[derive(Clone, Copy)]
enum RawClip {
    None,
    HardRounded,
    Rounded,
    HardPath,
}

fn raw_clip(renderer: &mut WgpuRenderer, factor: f32, clip: RawClip) -> ([[u8; 4]; 4], usize) {
    let mut canvas = Canvas::default();
    canvas.save_transform(Transform::translation(Offset::new(4., 4.)));
    canvas.save_transform(Transform::scale(factor));
    let size = 40. / factor;
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(size, size));
    match clip {
        RawClip::Rounded => canvas.save_clip_rrect(RRect::uniform(rect, 12. / factor)),
        RawClip::HardRounded => canvas.save_clip_rrect_with_coverage(
            RRect::uniform(rect, 12. / factor),
            ClipCoverage::HardEdge,
        ),
        RawClip::HardPath => {
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
        RawClip::None => {}
    }
    canvas.rect(rect, Color::WHITE);
    if !matches!(clip, RawClip::None) {
        canvas.restore();
    }
    canvas.restore();
    canvas.restore();
    clip_samples(renderer, &canvas.finish())
}

fn oval_clip(renderer: &mut WgpuRenderer, clip: Option<Clip>) -> ([[u8; 4]; 4], usize) {
    let oval = ClipOval::new(Widget::box_(Size::new(40., 28.), Color::WHITE));
    let oval = match clip {
        Some(clip) => oval.clip_behavior(clip),
        None => oval,
    };
    let positioned = Positioned::new(oval)
        .left(4.)
        .top(4.)
        .width(40.)
        .height(28.);
    let mut runtime = Runtime::new(Stack::new([Widget::from(positioned)]).into()).unwrap();
    let (list, _) = runtime
        .run_frame(Constraints::tight(Size::new(48., 48.)))
        .unwrap();
    clip_samples(renderer, &list)
}

fn clip_samples(renderer: &mut WgpuRenderer, list: &DisplayList) -> ([[u8; 4]; 4], usize) {
    renderer.request_capture();
    assert!(matches!(
        renderer.render(list, 1.).unwrap(),
        FrameOutcome::Presented(_)
    ));
    let frame = renderer.take_capture().unwrap().unwrap();
    let pixel = |(x, y)| -> [u8; 4] {
        let index = ((y * frame.width + x) * 4) as usize;
        frame.rgba8[index..index + 4].try_into().unwrap()
    };
    let controls = [(4, 4), (16, 16), (34, 16), (45, 16)].map(pixel);
    let partial = (4..16)
        .flat_map(|y| (4..16).map(move |x| pixel((x, y))))
        .filter(|pixel| pixel[1] > 0 && pixel[1] < 255)
        .count();
    (controls, partial)
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
        let unscaled_rrect = raw_clip(renderer, 1., RawClip::HardRounded);
        let scaled_rrect = raw_clip(renderer, 2., RawClip::HardRounded);
        let unscaled_path = raw_clip(renderer, 1., RawClip::HardPath);
        let scaled_path = raw_clip(renderer, 2., RawClip::HardPath);
        assert_eq!(
            raw_clip(renderer, 2., RawClip::None).0,
            [WHITE, WHITE, WHITE, BLACK]
        );
        assert_eq!(
            [unscaled_rrect, scaled_rrect, unscaled_path, scaled_path],
            [(expected, 0); 4],
            "the scissor follows the full transform for rounded and path clips"
        );
        let ordinary = raw_clip(renderer, 1., RawClip::Rounded);
        let scaled_ordinary = raw_clip(renderer, 2., RawClip::Rounded);
        assert_eq!((ordinary.0, scaled_ordinary.0), (expected, expected));
        assert!(
            ordinary.1 > 0 && scaled_ordinary.1 == ordinary.1,
            "default rounded Canvas clips should antialias under scaling: {ordinary:?}, {scaled_ordinary:?}"
        );
        let hard_oval = oval_clip(renderer, Some(Clip::HardEdge));
        let default_oval = oval_clip(renderer, None);
        assert_eq!(hard_oval, (expected, 0));
        assert_eq!(default_oval.0, expected);
        assert!(
            default_oval.1 > 0,
            "the default elliptical clip should antialias: {default_oval:?}"
        );
    });
}
