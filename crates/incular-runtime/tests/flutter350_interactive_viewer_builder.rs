use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Clip, Constraints, EdgeInsets, TextDirection};
use incular_core::{Color, Offset, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Column, InteractiveViewer, SizedBox, TransformationController, Widget,
    internal::{PointerData, PointerDataChange as Change, RenderKind, WidgetTree},
};

fn root_bounds() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn render_kinds(tree: &WidgetTree) -> Vec<(Option<bool>, &RenderKind)> {
    let mut pending = vec![tree.root().unwrap()];
    let mut out = Vec::new();
    while let Some(element) = pending.pop() {
        if let Some(id) = tree.render_id(element)
            && let Some(kind) = tree.render_object_kind(id)
        {
            out.push((tree.layout_builder_uses_constraints(id), kind));
        }
        if let Some(children) = tree.children(element) {
            pending.extend(children.iter().copied());
        }
    }
    out
}

#[test]
fn disabled_clip_remains_mounted_but_only_builder_has_layout_builder() {
    for clip in [Clip::None, Clip::HardEdge] {
        for build in [false, true] {
            let viewer = if build {
                InteractiveViewer::builder(|_, _| SizedBox::square(200.).into())
            } else {
                InteractiveViewer::new(SizedBox::square(200.)).constrained(clip != Clip::None)
            }
            .clip_behavior(clip)
            .min_scale(1.)
            .max_scale(1.);
            let mut runtime = Runtime::new(Center::new(viewer).into()).unwrap();
            runtime.run_frame(root_bounds()).unwrap();
            let kinds = render_kinds(runtime.tree());
            assert_eq!(
                kinds
                    .iter()
                    .filter(|(constraint_dependent, _)| *constraint_dependent == Some(true))
                    .count(),
                usize::from(build)
            );
            let clips: Vec<_> = kinds
                .iter()
                .filter_map(|&(_, kind)| match kind {
                    RenderKind::ClipRect { clip_behavior, .. } => Some(*clip_behavior),
                    _ => None,
                })
                .collect();
            assert_eq!(clips, [clip]);
            let tree = runtime.tree();
            let root = tree.render_id(tree.root().unwrap()).unwrap();
            if build {
                assert_eq!(tree.max_intrinsic_width(root, 600.), None);
            } else if clip == Clip::HardEdge {
                assert_eq!(tree.max_intrinsic_width(root, 600.), Some(200.));
            }
        }
    }
}

#[test]
fn builder_repaints_the_same_keyed_rows_when_the_visible_scene_moves() {
    let controller = TransformationController::new();
    let reports = Rc::new(RefCell::new(Vec::new()));
    let observations = reports.clone();
    let green = Color::rgba(0, 255, 0, 255);
    let red = Color::rgba(255, 0, 0, 255);
    let viewport = InteractiveViewer::builder(move |_, viewport| {
        let scene = viewport.bounds();
        observations.borrow_mut().push(scene);
        let bottom = scene.origin.y + scene.size.height;
        Column::new((0..10).map(|i| {
            let top = i as f32 * 10.;
            let end = top + 10.;
            let visible = (end >= scene.origin.y && end <= bottom)
                || (top >= scene.origin.y && top <= bottom);
            Widget::box_(Size::new(50., 10.), if visible { green } else { red })
                .with_key(1000 + i as u64)
        }))
        .text_direction(TextDirection::Ltr)
        .into()
    })
    .boundary_margin(EdgeInsets::all(f32::INFINITY))
    .scale_enabled(false)
    .transformation_controller(controller.clone());
    let root = Center::new(SizedBox::new().width(200.).height(50.).child(viewport));
    let mut runtime = Runtime::new(root.into()).unwrap();
    let t = Instant::now();
    runtime.run_frame_at(root_bounds(), t).unwrap();
    let kind = |runtime: &Runtime, i: usize| {
        let tree = runtime.tree();
        let id = tree.element_with_key(&(1000 + i as u64).into()).unwrap();
        let render = tree.render_id(id).unwrap();
        let Some(RenderKind::Box { color, .. }) = tree.render_object_kind(render) else {
            panic!("row is not a box")
        };
        (id, *color)
    };
    for i in 0..10 {
        assert_eq!(kind(&runtime, i).1, if i < 6 { green } else { red });
    }
    let before = (0..10).map(|i| kind(&runtime, i).0).collect::<Vec<_>>();
    assert_eq!(reports.borrow().last().unwrap().origin, Offset::ZERO);
    assert_eq!(reports.borrow().last().unwrap().size.height, 50.);

    for (index, (change, y)) in [
        (Change::Down, 290.),
        (Change::Move, 275.),
        (Change::Up, 275.),
    ]
    .into_iter()
    .enumerate()
    {
        let at = t + Duration::from_millis(index as u64 * 150 + 20);
        runtime.handle_pointer_data_packet(
            1,
            1.,
            [PointerData {
                view_id: 1,
                device: 1,
                pointer: 1,
                physical_position: Offset::new(320., y),
                ..PointerData::new(change, at)
            }],
        );
        runtime.run_frame_at(root_bounds(), at).unwrap();
    }
    assert_eq!(
        controller.value().translation_offset(),
        Offset::new(0., -15.)
    );
    assert_eq!(reports.borrow().last().unwrap().origin.y, 15.);
    assert_eq!(reports.borrow().last().unwrap().size.height, 50.);
    for (i, before) in before.into_iter().enumerate() {
        assert_eq!(
            kind(&runtime, i),
            (before, if i > 0 && i < 7 { green } else { red })
        );
    }
}
