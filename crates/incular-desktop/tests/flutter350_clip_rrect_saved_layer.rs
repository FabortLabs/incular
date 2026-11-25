//! Windows wgpu readback specifically for a retained rounded save-layer clip.
#![cfg(target_os = "windows")]
#[path = "support/clip_rrect_raster.rs"]
mod fixture;
use fixture::{BLACK, WHITE, capture, with_renderer};
use incular_config::Clip;
#[test]
fn isolated_overlap_has_smooth_edges_without_an_earlier_paint_fringe() {
    with_renderer(|renderer| {
        let hard = capture(renderer, Some(Clip::HardEdge), true);
        let saved = capture(renderer, Some(Clip::AntiAliasWithSaveLayer), true);
        assert_eq!((saved.corner, saved.center), (BLACK, WHITE));
        assert_eq!(hard.partial, 0, "hard-edge control");
        assert!(saved.partial > 0, "save-layer must smooth edges: {saved:?}");
        assert_eq!(
            saved.red_fringe, 0,
            "first paint must not show through the edge"
        );
    });
}
