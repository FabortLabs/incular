//! Native Windows wgpu readback for both pinned ClipRRect raster modes.
#![cfg(target_os = "windows")]
#[path = "support/clip_rrect_raster.rs"]
mod fixture;
use fixture::{BLACK, WHITE, capture, with_renderer};
use incular_config::Clip;
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
    });
}
