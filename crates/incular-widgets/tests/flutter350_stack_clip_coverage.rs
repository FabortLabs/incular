use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_widgets::{
    Center, Directionality, IndexedStack, Positioned, SizedBox, Stack, TextDirection, Widget,
    internal::{Key, RenderKind, WidgetTree},
};

fn check(indexed: bool) {
    let stack = Key::Value(42);
    let build = |clip| {
        let children = [
            SizedBox::new().width(100.).height(100.).into(),
            Widget::from(
                Positioned::new(Widget::box_(Size::new(180., 180.), Color::WHITE))
                    .left(0.)
                    .top(0.),
            ),
        ];
        let stack = if indexed {
            let widget = IndexedStack::new(children).index(Some(1));
            Widget::from(match clip {
                Some(clip) => widget.clip_behavior(clip),
                None => widget,
            })
        } else {
            let widget = Stack::new(children);
            Widget::from(match clip {
                Some(clip) => widget.clip_behavior(clip),
                None => widget,
            })
        }
        .with_key(stack.clone());
        Widget::from(Directionality::new(TextDirection::Ltr, Center::new(stack)))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(None)).unwrap();
    let mut render = None;
    let constraints = Constraints::tight(Size::new(800., 600.));
    for (step, requested) in [
        None,
        None,
        Some(Clip::AntiAlias),
        Some(Clip::AntiAliasWithSaveLayer),
        Some(Clip::None),
        Some(Clip::HardEdge),
    ]
    .into_iter()
    .enumerate()
    {
        if step > 0 {
            tree.update(root, build(requested)).unwrap();
        }
        tree.layout(constraints).unwrap();
        let element = tree.element_with_key(&stack).unwrap();
        let id = tree.render_id(element).unwrap();
        assert_eq!(*render.get_or_insert(id), id);
        let clip = match tree.render_object_kind(id).unwrap() {
            RenderKind::Stack { stack } => stack.clip_behavior,
            RenderKind::IndexedStack { clip_behavior, .. } => *clip_behavior,
            other => panic!("unexpected retained kind: {other:?}"),
        };
        assert_eq!(clip, requested.unwrap_or(Clip::HardEdge));
        let expected_box = Rect::from_origin_size(Offset::new(350., 250.), Size::new(100., 100.));
        assert_eq!(tree.element_bounds(element), Some(expected_box));
        let actual = tree
            .paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushClip { rect } => Some((*rect, ClipCoverage::HardEdge, false)),
                PaintCommand::PushClipRRect {
                    rrect,
                    coverage,
                    isolation,
                } => Some((rrect.rect, *coverage, isolation.is_some())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let expected = match clip {
            Clip::None => None,
            Clip::HardEdge => Some((expected_box, ClipCoverage::HardEdge, false)),
            Clip::AntiAlias => Some((expected_box, ClipCoverage::AntiAlias, false)),
            Clip::AntiAliasWithSaveLayer => {
                Some((expected_box, ClipCoverage::AntiAliasWithSaveLayer, true))
            }
        };
        assert_eq!(actual, expected.into_iter().collect::<Vec<_>>(), "{clip:?}");
    }
}

#[test]
fn stack_paints_using_the_selected_coverage_after_each_update() {
    check(false);
}

#[test]
fn indexed_stack_paints_using_the_selected_coverage_after_each_update() {
    check(true);
}
