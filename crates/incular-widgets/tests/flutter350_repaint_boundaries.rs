use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Offset, Size, time::Instant};
use incular_rendering::PaintCommand;
use incular_widgets::{
    GlobalKey, Opacity, Padding, RepaintBoundary, SizedBox, Stack, Widget, internal::WidgetTree,
};

fn frame(tree: &mut WidgetTree) -> incular_rendering::DisplayList {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let result = tree.try_paint().unwrap();
    tree.update_semantics();
    result
}

#[test]
fn nested_repaint_boundaries_survive_consecutive_opacity_updates() {
    for visible in [false, true] {
        let outer = GlobalKey::new();
        let inner = GlobalKey::new();
        let scene = |[a, b, c]: [f32; 3]| {
            let leaf = if visible {
                Widget::box_(Size::new(800., 600.), Color::WHITE)
            } else {
                SizedBox::shrink().into()
            };
            let inner_boundary =
                Widget::from(RepaintBoundary::new(Opacity::new(c, leaf))).with_key(inner);
            let outer_boundary =
                Widget::from(RepaintBoundary::new(Opacity::new(b, inner_boundary))).with_key(outer);
            Widget::from(Opacity::new(a, outer_boundary))
        };

        let mut tree = WidgetTree::new();
        let root = tree.mount(scene([1., 1., 1.])).unwrap();
        let _ = frame(&mut tree);
        let outer_element = tree.element_with_key(&outer.into()).unwrap();
        let inner_element = tree.element_with_key(&inner.into()).unwrap();
        let outer_render = tree.render_id(outer_element);
        let inner_render = tree.render_id(inner_element);

        for (next, expected) in [
            ([1., 1., 0.9], &[0.9][..]),
            ([0.8, 1., 0.8], &[0.8, 0.8][..]),
            ([0.7, 0.7, 0.7], &[0.7, 0.7, 0.7][..]),
        ] {
            tree.update(root, scene(next)).unwrap();
            let list = frame(&mut tree);
            assert_eq!(tree.element_with_key(&outer.into()), Some(outer_element));
            assert_eq!(tree.element_with_key(&inner.into()), Some(inner_element));
            assert_eq!(tree.render_id(outer_element), outer_render);
            assert_eq!(tree.render_id(inner_element), inner_render);
            if visible {
                let alphas: Vec<_> = list
                    .commands()
                    .iter()
                    .filter_map(|op| match op {
                        PaintCommand::PushOpacity { alpha, .. } if *alpha != 1. => Some(*alpha),
                        _ => None,
                    })
                    .collect();
                assert_eq!(alphas, expected);
            }
        }
    }
}

#[test]
fn a_dirty_repaint_boundary_keeps_its_render_identity_after_reparenting() {
    let key = GlobalKey::new();
    let boundary = |color| {
        Widget::from(RepaintBoundary::new(Widget::box_(Size::new(1., 1.), color))).with_key(key)
    };
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Stack::new([boundary(Color::WHITE)]).into())
        .unwrap();
    let _ = frame(&mut tree);
    let element = tree.element_with_key(&key.into()).unwrap();
    let render = tree.render_id(element).unwrap();
    assert_eq!(tree.parent(element), Some(root));

    tree.update(element, boundary(Color::BLACK)).unwrap();
    tree.update(
        root,
        Stack::new([Padding::new(EdgeInsets::all(50.), boundary(Color::BLACK))]).into(),
    )
    .unwrap();
    let display = frame(&mut tree);
    assert_eq!(tree.element_with_key(&key.into()), Some(element));
    assert_eq!(tree.render_id(element), Some(render));
    assert_ne!(tree.parent(element), Some(root));
    assert_eq!(
        tree.element_bounds(element).unwrap().origin,
        Offset::new(50., 50.)
    );
    assert!(display.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::BLACK,
            ..
        }
    )));
    assert!(!display.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::WHITE,
            ..
        }
    )));
}
