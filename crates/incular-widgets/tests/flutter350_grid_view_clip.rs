//! Compare Flutter's configured clipping policy with clips actually painted.

use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_widgets::{Directionality, GridView, SliverGridDelegate, Widget, internal::WidgetTree};

fn grid(count: usize, clip: Option<Clip>) -> Widget {
    let view = GridView::new(
        SliverGridDelegate::fixed_cross_axis_count(3),
        (0..count).map(|_| Widget::box_(Size::new(50., 2000.), Color::WHITE)),
    );
    let view = match clip {
        None => view,
        Some(clip) => view.clip_behavior(clip),
    };
    Directionality::new(TextDirection::Ltr, view).into()
}

fn clips(tree: &mut WidgetTree) -> Vec<ClipCoverage> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => {
                assert_eq!(
                    *rect,
                    Rect::from_origin_size(Offset::ZERO, Size::new(800., 600.))
                );
                Some(ClipCoverage::HardEdge)
            }
            PaintCommand::PushClipRRect {
                rrect, coverage, ..
            } => {
                assert_eq!(
                    rrect.rect,
                    Rect::from_origin_size(Offset::ZERO, Size::new(800., 600.))
                );
                Some(*coverage)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn default_hard_edge_grid_does_not_clip_when_it_fits() {
    let mut tree = WidgetTree::new();
    tree.mount(grid(1, None)).unwrap();
    let size = Constraints::tight(Size::new(800., 600.));
    tree.layout(size).unwrap();
    assert!(clips(&mut tree).is_empty());
}

#[test]
fn overflowing_grid_paints_the_replaced_clip_policy_and_removes_it_when_fit() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(grid(12, None)).unwrap();
    let size = Constraints::tight(Size::new(800., 600.));
    tree.layout(size).unwrap();
    assert_eq!(clips(&mut tree), [ClipCoverage::HardEdge]);
    for (clip, want) in [
        (Clip::AntiAlias, vec![ClipCoverage::AntiAlias]),
        (
            Clip::AntiAliasWithSaveLayer,
            vec![ClipCoverage::AntiAliasWithSaveLayer],
        ),
        (Clip::None, Vec::new()),
        (Clip::HardEdge, vec![ClipCoverage::HardEdge]),
    ] {
        tree.update(root, grid(12, Some(clip))).unwrap();
        tree.layout(size).unwrap();
        assert_eq!(clips(&mut tree), want, "{clip:?}");
    }
    tree.update(root, grid(1, None)).unwrap();
    tree.layout(size).unwrap();
    assert!(clips(&mut tree).is_empty());
}

#[test]
fn a_fitting_grid_clips_only_during_overscroll() {
    use incular_scroll::{ScrollController, ScrollPhysics};

    let controller = ScrollController::new();
    let view = GridView::count(3, [Widget::box_(Size::new(50., 50.), Color::WHITE)])
        .controller(controller.clone())
        .physics(ScrollPhysics::BOUNCING.always_scrollable());
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, view).into())
        .unwrap();
    let size = Constraints::tight(Size::new(800., 600.));
    tree.layout(size).unwrap();
    assert!(clips(&mut tree).is_empty());
    assert!(controller.jump_to(-12.));
    tree.layout(size).unwrap();
    assert_eq!(clips(&mut tree), [ClipCoverage::HardEdge]);
    assert!(controller.jump_to(0.));
    tree.layout(size).unwrap();
    assert!(clips(&mut tree).is_empty());
}

#[test]
fn grid_builder_uses_antialias_coverage_when_overflowing() {
    let view = GridView::builder(10, SliverGridDelegate::fixed_cross_axis_count(3), |_| {
        Widget::box_(Size::new(50., 2000.), Color::WHITE)
    })
    .clip_behavior(Clip::AntiAlias);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, view).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(clips(&mut tree), [ClipCoverage::AntiAlias]);
}
