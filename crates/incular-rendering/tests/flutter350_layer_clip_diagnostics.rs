use std::sync::Arc;

use incular_core::{Offset, Rect, Size};
use incular_rendering::{ClipCoverage, CornerRadii, FillRule, LayerId, LayerTree, Path, RRect};

fn unit() -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(1., 1.))
}

fn debug_clip(build: impl FnOnce(&mut LayerTree) -> LayerId, expected: &str) {
    let mut tree = LayerTree::new();
    let layer = build(&mut tree);
    tree.set_root(layer);
    let debug = tree.debug_tree();
    let expected = format!("clipBehavior: {expected}");
    assert_eq!(
        debug
            .lines()
            .find_map(|line| line.split_once(" clipBehavior: "))
            .map(|(_, behavior)| format!("clipBehavior: {behavior}")),
        Some(expected),
        "{debug}"
    );
}

const ISOLATED: ClipCoverage = ClipCoverage::AntiAliasWithSaveLayer;

#[test]
fn rectangular_layer_describes_hard_edges_by_default_and_reports_explicit_isolation() {
    debug_clip(|tree| tree.create_clip_rect(unit()), "Clip.hardEdge");
    debug_clip(
        |tree| tree.create_clip_rect_with_coverage(unit(), ISOLATED),
        "Clip.antiAliasWithSaveLayer",
    );
}

#[test]
fn rounded_layer_describes_antialiasing_by_default_and_reports_explicit_isolation() {
    let shape = RRect::uniform(unit(), 0.2);
    debug_clip(|tree| tree.create_clip_rrect(shape), "Clip.antiAlias");
    debug_clip(
        |tree| tree.create_clip_rrect_with_coverage(shape, ISOLATED),
        "Clip.antiAliasWithSaveLayer",
    );
}

#[test]
fn superellipse_layer_describes_antialiasing_by_default_and_reports_explicit_isolation() {
    let radii = CornerRadii::uniform(0.2);
    debug_clip(
        |tree| tree.create_clip_rsuperellipse(unit(), radii),
        "Clip.antiAlias",
    );
    debug_clip(
        |tree| tree.create_clip_rsuperellipse_with_coverage(unit(), radii, ISOLATED),
        "Clip.antiAliasWithSaveLayer",
    );
}

#[test]
fn path_layer_describes_antialiasing_by_default_and_reports_explicit_isolation() {
    let mut builder = Path::builder();
    builder
        .move_to(Offset::ZERO)
        .line_to(Offset::new(1., 1.))
        .line_to(Offset::new(0., 1.))
        .close();
    let path = Arc::new(builder.build());
    debug_clip(
        |tree| tree.create_clip_path(path.clone(), FillRule::NonZero),
        "Clip.antiAlias",
    );
    debug_clip(
        |tree| tree.create_clip_path_with_coverage(path, FillRule::NonZero, ISOLATED),
        "Clip.antiAliasWithSaveLayer",
    );
}
