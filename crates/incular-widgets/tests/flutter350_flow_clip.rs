use std::{cell::Cell, rc::Rc};

use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_widgets::{
    Flow, FlowDelegate, FlowPaintingContext, Widget,
    internal::{Key, RenderKind, WidgetTree},
};

struct Shifted(Rc<Cell<usize>>);
impl FlowDelegate for Shifted {
    fn get_constraints_for_child(&self, _: usize, c: Constraints) -> Constraints {
        self.0.set(self.0.get() + 1);
        c.loosen()
    }

    fn paint_children(&self, context: &mut FlowPaintingContext<'_>) {
        context.paint_child(0, Transform::translation(Offset::new(0., 50.)), 0.2);
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

fn exercise_modes(wrapped: bool) {
    let key = Key::Value(1);
    let measures = Rc::new(Cell::new(0));
    let build = |clip| {
        let children = [Widget::box_(Size::new(80., 80.), Color::WHITE).with_key(key.clone())];
        let flow = if wrapped {
            Flow::new(Shifted(measures.clone()), children)
        } else {
            Flow::unwrapped(Shifted(measures.clone()), children)
        };
        let flow = match clip {
            Some(clip) => flow.clip_behavior(clip),
            None => flow,
        };
        Widget::from(flow)
    };
    let view = Size::new(80., 80.);
    let box_ = Rect::from_origin_size(Offset::ZERO, view);
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(None)).unwrap();
    let mut base = None;
    let mut old_layouts = None;
    for (index, requested) in [
        None,
        Some(Clip::None),
        Some(Clip::HardEdge),
        Some(Clip::AntiAlias),
        Some(Clip::AntiAliasWithSaveLayer),
    ]
    .into_iter()
    .enumerate()
    {
        if index != 0 {
            tree.update(root, build(requested)).unwrap();
        }
        tree.layout(Constraints::tight(view)).unwrap();
        let display = tree.paint();
        let mut child = tree.element_with_key(&key).unwrap();
        let mut boundaries = 0;
        let policy;
        loop {
            child = tree.parent(child).expect("flow is an ancestor");
            let render = tree.render_id(child).unwrap();
            match tree.render_object_kind(render).unwrap() {
                RenderKind::RepaintBoundary => boundaries += 1,
                RenderKind::CustomLayout { delegate } => {
                    assert_eq!(base.get_or_insert(render), &render);
                    policy = delegate
                        .overflow_clip()
                        .expect("flow declares clip coverage");
                    break;
                }
                _ => {}
            }
        }
        assert_eq!(boundaries, usize::from(wrapped));
        assert_eq!(policy, requested.unwrap_or(Clip::HardEdge));
        let found = display
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
        let coverage = match policy {
            Clip::None => None,
            Clip::HardEdge => Some((box_, ClipCoverage::HardEdge, false)),
            Clip::AntiAlias => Some((box_, ClipCoverage::AntiAlias, false)),
            Clip::AntiAliasWithSaveLayer => {
                Some((box_, ClipCoverage::AntiAliasWithSaveLayer, true))
            }
        };
        assert_eq!(
            found,
            coverage.into_iter().collect::<Vec<_>>(),
            "{policy:?}"
        );
        let layouts = measures.get();
        assert_eq!(
            *old_layouts.get_or_insert(layouts),
            layouts,
            "clip updates do not measure Flow again"
        );
    }
}

#[test]
fn wrapped_flow_updates_clip_coverage_on_the_same_render_object() {
    exercise_modes(true);
}

#[test]
fn unwrapped_flow_updates_clip_without_adding_child_repaint_boundaries() {
    exercise_modes(false);
}
