//! FittedBox clips only actual visual overflow.
use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_widgets::{
    BoxFit, FittedBox, Widget,
    internal::{RenderKind, WidgetTree},
};

fn scene(fit: BoxFit, clip: Option<Clip>, child: Size) -> Widget {
    let fitted = FittedBox::new(Widget::box_(child, Color::WHITE)).fit(fit);
    match clip {
        Some(clip) => fitted.clip_behavior(clip).into(),
        None => fitted.into(),
    }
}

fn clips(tree: &mut WidgetTree) -> Vec<(Rect, ClipCoverage)> {
    tree.paint()
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

/// widgets/fitted_box_test.dart:423, plus actual coverage and identity checks.
#[test]
fn a_retained_fitted_box_updates_clipping_without_relayout() {
    let mut tree = WidgetTree::new();
    let child = Size::new(100., 40.);
    let size = Size::new(40., 40.);
    let root = tree.mount(scene(BoxFit::None, None, child)).unwrap();
    let render = tree.render_id(root).unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    let layout_count = tree.diagnostics().layouts;
    for (setting, expected) in [
        (None, None),
        (Some(Clip::AntiAlias), Some(ClipCoverage::AntiAlias)),
        (
            Some(Clip::AntiAliasWithSaveLayer),
            Some(ClipCoverage::AntiAliasWithSaveLayer),
        ),
        (Some(Clip::HardEdge), Some(ClipCoverage::HardEdge)),
        (Some(Clip::None), None),
    ] {
        tree.update(root, scene(BoxFit::None, setting, child))
            .unwrap();
        tree.layout(Constraints::tight(size)).unwrap();
        assert_eq!(tree.render_id(root), Some(render));
        assert_eq!(tree.diagnostics().layouts, layout_count);
        assert!(matches!(tree.render_object_kind(render),
            Some(RenderKind::FittedBox { clip_behavior, .. })
                if *clip_behavior == setting.unwrap_or(Clip::None)
        ));
        assert_eq!(
            clips(&mut tree),
            expected
                .map(|c| (Rect::from_origin_size(Offset::ZERO, size), c))
                .into_iter()
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn only_cropped_fits_emit_a_clip() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(
            BoxFit::None,
            Some(Clip::HardEdge),
            Size::new(10., 10.),
        ))
        .unwrap();
    for w in [10., 50., 100.] {
        for h in [10., 50., 100.] {
            for cw in [10., 50., 100.] {
                for ch in [10., 50., 100.] {
                    let bounds = Size::new(w, h);
                    tree.update(
                        root,
                        scene(BoxFit::None, Some(Clip::HardEdge), Size::new(cw, ch)),
                    )
                    .unwrap();
                    tree.layout(Constraints::tight(bounds)).unwrap();
                    assert_eq!(clips(&mut tree).len(), usize::from(cw > w || ch > h));
                }
            }
        }
    }
    for (fit, expected) in [
        (BoxFit::Contain, false),
        (BoxFit::Cover, true),
        (BoxFit::Fill, false),
        (BoxFit::ScaleDown, false),
        (BoxFit::FitWidth, false),
        (BoxFit::FitHeight, true),
    ] {
        tree.update(root, scene(fit, Some(Clip::HardEdge), Size::new(100., 50.)))
            .unwrap();
        tree.layout(Constraints::tight(Size::new(40., 40.)))
            .unwrap();
        assert_eq!(clips(&mut tree).len(), usize::from(expected), "{fit:?}");
    }
    for source in [Size::new(100., 50.), Size::new(50., 100.)] {
        for (fit, expected) in [(BoxFit::Cover, 1), (BoxFit::Contain, 0)] {
            tree.update(root, scene(fit, Some(Clip::HardEdge), source))
                .unwrap();
            tree.layout(Constraints::tight(Size::new(40., 40.)))
                .unwrap();
            assert_eq!(clips(&mut tree).len(), expected, "{fit:?} with {source:?}");
        }
    }
}
