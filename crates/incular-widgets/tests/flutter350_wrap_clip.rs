//! Pinned RenderWrap clipping and retained clip updates.
use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_widgets::{Widget, Wrap, internal::WidgetTree};

const HOST: Size = Size {
    width: 800.,
    height: 600.,
};

fn scene(count: usize, mode: Option<Clip>) -> Widget {
    let wrap = Wrap::new(
        (0..count).map(|_| Widget::box_(Size::new(500., 500.), Color::rgba(40, 70, 150, 255))),
    )
    .text_direction(TextDirection::Ltr);
    if let Some(mode) = mode {
        wrap.clip_behavior(mode).into()
    } else {
        wrap.into()
    }
}

fn clips(tree: &mut WidgetTree) -> Vec<(Rect, ClipCoverage)> {
    tree.layout(Constraints::tight(HOST)).unwrap();
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => Some((*rect, ClipCoverage::HardEdge)),
            PaintCommand::PushClipRRect { rrect, coverage, .. } => Some((rrect.rect, *coverage)),
            _ => None,
        })
        .collect()
}

#[test]
fn an_overflowing_wrap_clips_only_when_explicitly_enabled() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(1, None)).unwrap();
    let rect = Rect::from_origin_size(Offset::ZERO, HOST);
    assert!(clips(&mut tree).is_empty());

    tree.update(root, scene(2, None)).unwrap();
    assert!(clips(&mut tree).is_empty());
    tree.update(root, scene(2, Some(Clip::HardEdge))).unwrap();
    assert_eq!(clips(&mut tree), [(rect, ClipCoverage::HardEdge)]);

    tree.update(root, scene(1, Some(Clip::HardEdge))).unwrap();
    assert!(clips(&mut tree).is_empty());
}

#[test]
fn updating_wrap_clip_modes_updates_the_same_render_object_and_clears_old_clips() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(0, None)).unwrap();
    let original = tree.render_id(root).unwrap();
    assert!(clips(&mut tree).is_empty());

    tree.update(root, scene(0, Some(Clip::AntiAlias))).unwrap();
    assert_eq!(tree.render_id(root), Some(original));
    assert!(clips(&mut tree).is_empty());

    let rect = Rect::from_origin_size(Offset::ZERO, HOST);
    for (mode, expected) in [
        (Clip::AntiAlias, vec![(rect, ClipCoverage::AntiAlias)]),
        (
            Clip::AntiAliasWithSaveLayer,
            vec![(rect, ClipCoverage::AntiAliasWithSaveLayer)],
        ),
        (Clip::None, vec![]),
    ] {
        tree.update(root, scene(2, Some(mode))).unwrap();
        assert_eq!(tree.render_id(root), Some(original));
        assert_eq!(clips(&mut tree), expected);
    }
}
