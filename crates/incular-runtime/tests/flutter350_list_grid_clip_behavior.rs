//! Retained viewport settings and the actual submitted clip coverage.

use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, GridView, ListView, SizedBox, SliverChildBuilderDelegate, SliverGridDelegate,
    Widget,
    internal::{Key, RenderKind},
};

const VIEW: Key = Key::Value(54);
const BOUNDS: Size = Size {
    width: 800.,
    height: 600.,
};

fn card() -> Widget {
    Widget::box_(Size::new(800., 2000.), Color::WHITE)
}

fn scene(view: impl Into<Widget>) -> Widget {
    Directionality::new(TextDirection::Ltr, view.into().with_key(VIEW)).into()
}

fn viewport(runtime: &Runtime) -> (impl Copy + Eq + std::fmt::Debug + use<>, Clip) {
    let tree = runtime.tree();
    let id = tree
        .render_id(tree.element_with_key(&VIEW).unwrap())
        .unwrap();
    let Some(RenderKind::SliverViewport { config }) = tree.render_object_kind(id) else {
        panic!("not a sliver viewport");
    };
    (id, config.clip_behavior())
}

fn clips(paint: &DisplayList) -> Vec<(Rect, ClipCoverage)> {
    paint
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => Some((*rect, ClipCoverage::HardEdge)),
            PaintCommand::PushClipRRect {
                rrect, coverage, ..
            } => Some((rrect.rect, *coverage)),
            _ => None,
        })
        .collect()
}

fn check_clip_rebuild(make: impl Fn(Option<Clip>) -> Widget) {
    let constraints = Constraints::tight(BOUNDS);
    let rect = Rect::from_origin_size(Offset::ZERO, BOUNDS);
    let mut runtime = Runtime::new(make(None)).unwrap();
    let (initial, _) = runtime.run_frame(constraints).unwrap();
    let (id, behavior) = viewport(&runtime);
    assert_eq!(behavior, Clip::HardEdge);
    assert_eq!(clips(&initial), [(rect, ClipCoverage::HardEdge)]);

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, make(Some(Clip::AntiAlias)))
        .unwrap();
    let (changed, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(viewport(&runtime), (id, Clip::AntiAlias));
    assert_eq!(clips(&changed), [(rect, ClipCoverage::AntiAlias)]);

    runtime
        .schedule_update(root, make(Some(Clip::None)))
        .unwrap();
    let (unclipped, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(viewport(&runtime), (id, Clip::None));
    assert!(clips(&unclipped).is_empty());
}

#[test]
fn default_list_view_updates_its_retained_clip_and_the_painted_edge() {
    check_clip_rebuild(|clip| {
        let view = ListView::new([card()]);
        scene(match clip {
            Some(clip) => view.clip_behavior(clip),
            None => view,
        })
    });
}

#[test]
fn default_grid_view_updates_its_retained_clip_and_the_painted_edge() {
    check_clip_rebuild(|clip| {
        let view = GridView::new(
            SliverGridDelegate::fixed_cross_axis_count(3),
            (0..10).map(|_| card()),
        );
        scene(match clip {
            Some(clip) => view.clip_behavior(clip),
            None => view,
        })
    });
}

fn delegate(count: usize) -> SliverChildBuilderDelegate {
    SliverChildBuilderDelegate::new(|_| Some(card())).child_count(count)
}

#[test]
fn builder_custom_and_separated_lists_pass_clip_coverage_to_the_viewport() {
    for view in [
        ListView::builder(10, |_| card()),
        ListView::custom(delegate(1)),
        ListView::separated(10, |_| card(), |_| SizedBox::shrink()),
    ] {
        let mut runtime = Runtime::new(scene(view.clip_behavior(Clip::AntiAlias))).unwrap();
        let (paint, _) = runtime.run_frame(Constraints::tight(BOUNDS)).unwrap();
        assert_eq!(viewport(&runtime).1, Clip::AntiAlias);
        assert_eq!(
            clips(&paint),
            [(
                Rect::from_origin_size(Offset::ZERO, BOUNDS),
                ClipCoverage::AntiAlias
            )]
        );
    }
}

#[test]
fn grid_constructors_retain_clip_policy_without_clipping_nonoverflowing_tiles() {
    let fixed = || SliverGridDelegate::fixed_cross_axis_count(3);
    for (view, overflow) in [
        (GridView::builder(10, fixed(), |_| card()), true),
        (GridView::custom(fixed(), delegate(1)), false),
        (GridView::count(3, [card()]), false),
        (GridView::extent(1000., [card()]), true),
    ] {
        let mut runtime = Runtime::new(scene(view.clip_behavior(Clip::AntiAlias))).unwrap();
        let (paint, _) = runtime.run_frame(Constraints::tight(BOUNDS)).unwrap();
        assert_eq!(viewport(&runtime).1, Clip::AntiAlias);
        let expected = [(
            Rect::from_origin_size(Offset::ZERO, BOUNDS),
            ClipCoverage::AntiAlias,
        )];
        assert_eq!(clips(&paint), if overflow { &expected[..] } else { &[] });
    }
}
