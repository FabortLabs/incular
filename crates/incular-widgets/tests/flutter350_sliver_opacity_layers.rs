use std::time::Duration;

use incular_animation::AnimationController;
use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, LayerId, PaintCommand};
use incular_widgets::{
    CustomScrollView, Directionality, Sliver, SliverFadeTransition, SliverOpacity,
    SliverToBoxAdapter, Widget, internal::WidgetTree,
};

fn child(color: Color) -> SliverToBoxAdapter {
    SliverToBoxAdapter::new(Widget::box_(Size::new(1., 1.), color))
}

fn scene(sliver: impl Sliver + 'static) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>]),
    )
    .into()
}

fn paint(tree: &mut WidgetTree, at: Instant) -> DisplayList {
    tree.layout(Constraints::tight(Size::new(10., 10.)))
        .unwrap();
    tree.update_compositor(at).unwrap();
    tree.paint()
}

fn opacity(display: &DisplayList) -> (LayerId, f32, u64) {
    let records: Vec<_> = display
        .commands()
        .iter()
        .filter_map(|op| match op {
            PaintCommand::PushOpacity {
                layer,
                alpha,
                generation,
                ..
            } => Some((*layer, *alpha, *generation)),
            _ => None,
        })
        .collect();
    assert_eq!(records.len(), 1);
    records[0]
}

fn amount(widget: Widget, byte: u8) {
    let mut tree = WidgetTree::new();
    tree.mount(widget).unwrap();
    let display = paint(&mut tree, Instant::now());
    let (_, alpha, generation) = opacity(&display);
    assert_eq!(alpha, f32::from(byte) / 255.);
    assert_eq!(
        display.commands().iter().any(|op| matches!(
            op,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )),
        byte > 0
    );
    assert_eq!(generation != 0, byte > 0);
}

#[test]
fn sliver_opacity_zero_has_no_source_draw_and_full_opacity_does() {
    amount(scene(SliverOpacity::new(0., child(Color::WHITE))), 0);
    amount(scene(SliverOpacity::new(1., child(Color::WHITE))), 255);
}

#[test]
fn animated_sliver_opacity_zero_partial_and_full_match_compositing() {
    for (value, byte) in [(0., 0_u8), (0.5, 128), (1., 255)] {
        let animation = AnimationController::new(Duration::from_secs(1));
        animation.set_value(value);
        amount(
            scene(SliverFadeTransition::from_animation(
                animation,
                child(Color::WHITE),
            )),
            byte,
        );
    }
}

fn repaint_keeps_layer(build: impl Fn(Color) -> Widget) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(Color::WHITE)).unwrap();
    let at = Instant::now();
    let first = opacity(&paint(&mut tree, at));
    tree.update(root, build(Color::BLACK)).unwrap();
    let changed = paint(&mut tree, at);
    let second = opacity(&changed);
    assert_eq!(first.0, second.0);
    assert_eq!(second.1, 128. / 255.);
    assert_ne!(first.2, second.2);
    assert!(changed.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::BLACK,
            ..
        }
    )));
}

#[test]
fn static_sliver_opacity_layer_survives_a_dirty_child_repaint() {
    repaint_keeps_layer(|color| scene(SliverOpacity::new(0.5, child(color))));
}

#[test]
fn animated_sliver_opacity_layer_survives_a_dirty_child_repaint() {
    let animation = AnimationController::new(Duration::from_secs(1));
    animation.set_value(0.5);
    repaint_keeps_layer(|color| {
        scene(SliverFadeTransition::from_animation(
            animation.clone(),
            child(color),
        ))
    });
}

#[test]
fn overlapping_sliver_rows_share_one_half_opacity_isolation() {
    let rows = incular_widgets::SliverFixedExtentList::new(2, 40., |index| {
        if index == 0 {
            incular_widgets::Transform::translation(
                incular_core::Offset::new(0., 20.),
                Widget::box_(Size::new(100., 40.), Color::WHITE),
            )
            .into()
        } else {
            Widget::box_(Size::new(100., 40.), Color::BLACK)
        }
    });
    let mut tree = WidgetTree::new();
    tree.mount(scene(SliverOpacity::new(0.5, rows))).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let list = tree.paint();
    let mut group_depth = 0;
    let mut groups = 0;
    let mut draws = 0;
    for op in list.commands() {
        match op {
            PaintCommand::PushOpacity { alpha, .. } => {
                assert_eq!(*alpha, 128. / 255.);
                groups += 1;
                group_depth += 1;
            }
            PaintCommand::PopOpacity => group_depth -= 1,
            PaintCommand::Rect {
                color: Color::WHITE | Color::BLACK,
                ..
            } => {
                assert_eq!(group_depth, 1);
                draws += 1;
            }
            _ => {}
        }
    }
    assert_eq!((groups, draws, group_depth), (1, 2, 0));
}

#[test]
fn transparent_group_keeps_requested_semantics_and_resumes_without_remounting() {
    for extent in [10., 100.] {
        transparent_group_resume(extent);
    }
}

fn transparent_group_resume(extent: f32) {
    use incular_widgets::{GlobalKey, Semantics};

    let key = GlobalKey::new();
    let scroll = incular_scroll::ScrollController::new();
    let build = |alpha, include| {
        let leaf = Widget::from(
            Semantics::new(Widget::box_(Size::new(100., 40.), Color::WHITE))
                .container(true)
                .label("fading child"),
        )
        .with_key(key);
        let sliver = SliverOpacity::new(alpha, SliverToBoxAdapter::new(leaf))
            .always_include_semantics(include);
        Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>])
                .controller(scroll.clone()),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(0., false)).unwrap();
    let at = Instant::now();
    let paint = |tree: &mut WidgetTree, at| {
        tree.layout(Constraints::tight(Size::new(extent, extent)))
            .unwrap();
        tree.update_compositor(at).unwrap();
        tree.paint()
    };
    let entry = |tree: &WidgetTree| {
        tree.semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("fading child"))
    };
    assert!(!paint(&mut tree, at).commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::WHITE,
            ..
        }
    )));
    let original_child = tree.element_with_key(&key.into()).unwrap();
    tree.update_semantics();
    assert!(!entry(&tree));

    tree.update(root, build(0., true)).unwrap();
    assert!(!paint(&mut tree, at).commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::WHITE,
            ..
        }
    )));
    tree.update_semantics();
    assert!(entry(&tree));

    tree.update(root, build(0.5, false)).unwrap();
    let list = paint(&mut tree, at);
    assert!(
        list.commands()
            .iter()
            .any(|op| matches!(op, PaintCommand::PushOpacity { .. })),
        "visible group must be submitted at viewport {extent}: {:?}; offset {:?}; child {:?}; diagnostics {:?}",
        list.commands(),
        scroll.offset(),
        tree.element_bounds(original_child),
        tree.diagnostics(),
    );
    assert_eq!(opacity(&list).1, 128. / 255.);
    assert!(list.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::WHITE,
            ..
        }
    )));
    assert_eq!(tree.element_with_key(&key.into()), Some(original_child));
    tree.update_semantics();
    assert!(entry(&tree));
}

#[test]
fn nested_clip_and_opacity_preserve_outer_to_inner_effect_order() {
    use incular_widgets::{BorderRadiusGeometry, SliverClipRRect};
    let build = |opacity_outside| {
        if opacity_outside {
            scene(SliverOpacity::new(
                0.5,
                SliverClipRRect::new(child(Color::WHITE))
                    .border_radius(BorderRadiusGeometry::circular(1.)),
            ))
        } else {
            scene(
                SliverClipRRect::new(SliverOpacity::new(0.5, child(Color::WHITE)))
                    .border_radius(BorderRadiusGeometry::circular(1.)),
            )
        }
    };
    for opacity_outside in [false, true] {
        let mut tree = WidgetTree::new();
        tree.mount(build(opacity_outside)).unwrap();
        let list = paint(&mut tree, Instant::now());
        let scopes: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|op| match op {
                PaintCommand::PushClipRRect { .. } => Some("clip"),
                PaintCommand::PushOpacity { .. } => Some("opacity"),
                _ => None,
            })
            .collect();
        assert_eq!(
            scopes,
            if opacity_outside {
                ["opacity", "clip"]
            } else {
                ["clip", "opacity"]
            }
        );
    }
}
