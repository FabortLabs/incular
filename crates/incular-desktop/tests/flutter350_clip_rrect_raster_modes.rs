//! Native Windows wgpu readback for pinned rectangular and elliptical clip modes.
#![cfg(target_os = "windows")]
#[path = "support/clip_rrect_raster.rs"]
mod fixture;
use std::sync::Arc;

use fixture::{BLACK, WHITE, capture, with_renderer};
use incular_config::{Clip, Constraints, StackFit};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{Canvas, ClipCoverage, DisplayList, FillRule, PathBuilder, RRect};
use incular_runtime::Runtime;
use incular_wgpu::{FrameOutcome, WgpuRenderer};
use incular_widgets::{ClipOval, ClipRect, ClipShape, CustomClipper, Positioned, Stack, Widget};

#[derive(Clone, Copy)]
enum RawClip {
    None,
    HardRounded,
    Rounded,
    HardPath,
}

type ClipSamples = ([[u8; 4]; 4], usize, usize);

fn raw_clip(renderer: &mut WgpuRenderer, factor: f32, clip: RawClip) -> ClipSamples {
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

fn oval_clip(
    renderer: &mut WgpuRenderer,
    clip: Option<Clip>,
    paint_red_first: bool,
) -> ClipSamples {
    let size = Size::new(40., 28.);
    let mut paints = Vec::new();
    if paint_red_first {
        paints.push(Widget::box_(size, Color::rgba(255, 0, 0, 255)));
    }
    paints.push(Widget::box_(size, Color::WHITE));
    let oval = ClipOval::new(Stack::new(paints).fit(StackFit::Expand));
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

fn clip_samples(renderer: &mut WgpuRenderer, list: &DisplayList) -> ClipSamples {
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
    let corner = (4..16).flat_map(|y| (4..16).map(move |x| pixel((x, y))));
    let partial = corner
        .clone()
        .filter(|pixel| pixel[1] > 0 && pixel[1] < 255)
        .count();
    let fringe = corner
        .filter(|pixel| u16::from(pixel[0]) > u16::from(pixel[1]) + 1)
        .count();
    (controls, partial, fringe)
}

fn assert_rotated_canvas_hard_clip(renderer: &mut WgpuRenderer) {
    for use_path in [true, false] {
        let mut canvas = Canvas::default();
        canvas.save_transform(Transform::translation(Offset::new(24., 24.)));
        canvas.save_transform(Transform::rotation(std::f32::consts::FRAC_PI_4));
        let rect = Rect::from_origin_size(Offset::new(-10., -10.), Size::new(20., 20.));
        if use_path {
            let mut path = PathBuilder::default();
            path.move_to(Offset::new(-10., -10.))
                .line_to(Offset::new(10., -10.))
                .line_to(Offset::new(10., 10.))
                .line_to(Offset::new(-10., 10.))
                .close();
            canvas.save_clip_path(Arc::new(path.build()), FillRule::NonZero);
        } else {
            canvas.save_clip(rect);
        }
        canvas.rect(
            Rect::from_origin_size(Offset::new(-40., -40.), Size::new(80., 80.)),
            Color::WHITE,
        );
        canvas.restore();
        canvas.restore();
        canvas.restore();
        renderer.request_capture();
        assert!(matches!(
            renderer.render(&canvas.finish(), 1.).unwrap(),
            FrameOutcome::Presented(_)
        ));
        let frame = renderer.take_capture().unwrap().unwrap();
        let samples = [(12, 12), (24, 24), (5, 5)].map(|(x, y)| {
            let index = ((y * frame.width + x) * 4) as usize;
            <[u8; 4]>::try_from(&frame.rgba8[index..index + 4]).unwrap()
        });
        assert_eq!(
            samples,
            [BLACK, WHITE, BLACK],
            "rotated Canvas clip, path={use_path}"
        );
    }
}

fn assert_fractional_clip_rect_modes(renderer: &mut WgpuRenderer) {
    struct FractionalBox;

    impl CustomClipper for FractionalBox {
        fn get_clip(&self, _: Size) -> ClipShape {
            ClipShape::Rect(Rect::from_origin_size(
                Offset::new(4.25, 4.25),
                Size::new(35.5, 31.5),
            ))
        }
    }

    let mut samples = |mode: Option<Clip>| {
        let size = Size::new(40., 40.);
        let paints = [
            Widget::box_(size, Color::rgba(255, 0, 0, 255)),
            Widget::box_(size, Color::WHITE),
        ];
        let clip =
            ClipRect::new(Stack::new(paints).fit(StackFit::Expand)).custom_clipper(FractionalBox);
        let clip = match mode {
            Some(mode) => clip.clip_behavior(mode),
            None => clip,
        };
        let positioned = Positioned::new(clip)
            .left(4.)
            .top(4.)
            .width(40.)
            .height(40.);
        let mut runtime = Runtime::new(Stack::new([Widget::from(positioned)]).into()).unwrap();
        let (list, _) = runtime
            .run_frame(Constraints::tight(Size::new(48., 48.)))
            .unwrap();
        clip_samples(renderer, &list)
    };
    let none = samples(Some(Clip::None));
    let hard = samples(None);
    let ordinary = samples(Some(Clip::AntiAlias));
    let saved = samples(Some(Clip::AntiAliasWithSaveLayer));
    assert_eq!(none, ([WHITE, WHITE, WHITE, BLACK], 0, 0));
    let clipped = [BLACK, WHITE, WHITE, BLACK];
    assert_eq!(hard, (clipped, 0, 0));
    assert_eq!((ordinary.0, saved.0), (clipped, clipped));
    assert!(
        ordinary.1 > 0 && ordinary.2 > 0 && saved.1 > 0 && saved.2 == 0,
        "expected per-draw and isolated fractional coverage: {ordinary:?}, {saved:?}"
    );
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
            [(expected, 0, 0); 4],
            "the scissor follows the full transform for rounded and path clips"
        );
        let ordinary = raw_clip(renderer, 1., RawClip::Rounded);
        let scaled_ordinary = raw_clip(renderer, 2., RawClip::Rounded);
        assert_eq!((ordinary.0, scaled_ordinary.0), (expected, expected));
        assert!(
            ordinary.1 > 0 && scaled_ordinary.1 == ordinary.1,
            "default rounded Canvas clips should antialias under scaling: {ordinary:?}, {scaled_ordinary:?}"
        );
        let hard_oval = oval_clip(renderer, Some(Clip::HardEdge), false);
        let default_oval = oval_clip(renderer, None, false);
        assert_eq!(hard_oval, (expected, 0, 0));
        assert_eq!(default_oval.0, expected);
        assert!(
            default_oval.1 > 0,
            "the default elliptical clip should antialias: {default_oval:?}"
        );
        let ordinary_oval = oval_clip(renderer, None, true);
        let isolated_oval = oval_clip(renderer, Some(Clip::AntiAliasWithSaveLayer), true);
        assert_eq!((ordinary_oval.0, isolated_oval.0), (expected, expected));
        assert!(
            ordinary_oval.1 > 0
                && ordinary_oval.2 > 0
                && isolated_oval.1 > 0
                && isolated_oval.2 == 0,
            "an isolated ellipse applies coverage once: {ordinary_oval:?}, {isolated_oval:?}"
        );
        assert_fractional_clip_rect_modes(renderer);
        assert_rotated_canvas_hard_clip(renderer);
    });
}
