use std::{cell::RefCell, rc::Rc};

use incular_config::{Alignment, Constraints};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{BlendMode, Brush, GradientStop, GradientStops, PaintCommand, Shader};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Align, BoxLinearGradient, ShaderCallback, ShaderMask, SizedBox};

fn gradient(bounds: Rect) -> Shader {
    let stops = GradientStops::new(vec![
        GradientStop {
            offset: 0.1,
            color: Color::rgba(255, 255, 255, 0),
        },
        GradientStop {
            offset: 0.35,
            color: Color::WHITE,
        },
    ]);
    Brush::LinearGradient(
        BoxLinearGradient::new(Alignment::TOP_CENTER, Alignment::BOTTOM_CENTER, stops)
            .create_shader(bounds, None)
            .unwrap(),
    )
}

#[test]
fn default_shader_mask_mounts_with_an_unpainted_sized_box() {
    let mut tree = WidgetTree::new();
    let mask = ShaderMask::new(gradient, SizedBox::square(100.));
    assert_eq!(mask.mode(), BlendMode::Modulate);
    let _root = tree.mount(mask.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let list = tree.paint();
    assert!(
        list.commands()
            .iter()
            .all(|op| !matches!(op, PaintCommand::Rect { .. }))
    );
}

#[test]
fn tightly_sized_centered_mask_passes_local_not_world_bounds() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let shader = ShaderCallback::new({
        let observed = observed.clone();
        move |bounds| {
            observed.borrow_mut().push(bounds);
            gradient(bounds)
        }
    });

    let centered_mask = |child: incular_widgets::Widget| {
        Align::new(
            Alignment::CENTER,
            SizedBox::square(400.).child(ShaderMask::new(shader.clone(), child)),
        )
    };
    let mut tree = WidgetTree::new();
    tree.mount(centered_mask(SizedBox::square(100.).into()).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
    let local = Rect::from_origin_size(Offset::ZERO, Size::new(400., 400.));
    assert_eq!(*observed.borrow(), [local]);

    // A separate painted control reaches the backend with the same shader
    // coordinate space and the independently translated mask placement.
    observed.borrow_mut().clear();
    let mut tree = WidgetTree::new();
    tree.mount(
        centered_mask(
            incular_widgets::ColoredBox::new(Color::WHITE, SizedBox::square(100.)).into(),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let list = tree.paint();
    assert_eq!(*observed.borrow(), [local]);
    let stages: Vec<_> = list
        .commands()
        .iter()
        .filter_map(|op| match op {
            PaintCommand::PushShaderMask {
                shader,
                blend_mode,
                mask_size,
                mask_transform,
                bounds,
                ..
            } => Some((shader, blend_mode, mask_size, mask_transform, bounds)),
            _ => None,
        })
        .collect();
    let [(shader, blend_mode, mask_size, transform, bounds)] = stages.as_slice() else {
        panic!("expected one mask stage, got {stages:?}");
    };
    let origin = Offset::new(200., 100.);
    assert_eq!(**shader, gradient(local));
    assert_eq!(**blend_mode, BlendMode::Modulate);
    assert_eq!(**mask_size, local.size);
    assert_eq!(transform.translation_offset(), origin);
    assert_eq!(**bounds, Rect::from_origin_size(origin, local.size));
}

#[test]
fn preserved_shader_tracks_reflection_even_when_the_mask_bounds_are_unchanged() {
    use incular_widgets::{Transform, Widget};
    use std::cell::Cell;

    let calls = Rc::new(Cell::new(0));
    let shader = ShaderCallback::new({
        let calls = calls.clone();
        move |bounds| {
            assert_eq!(
                bounds,
                Rect::from_origin_size(Offset::ZERO, Size::new(16., 16.))
            );
            calls.set(calls.get() + 1);
            Brush::Solid(Color::WHITE)
        }
    });
    let make = |x: f32, reflection: f32| -> Widget {
        Align::new(
            Alignment::TOP_LEFT,
            Transform::translation(
                Offset::new(x, 8.),
                Transform::scale_non_uniform(
                    reflection,
                    1.,
                    ShaderMask::new(
                        shader.clone(),
                        Widget::box_(Size::new(16., 16.), Color::WHITE),
                    ),
                ),
            ),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(8., 1.)).unwrap();
    let constraints = Constraints::tight(Size::new(64., 48.));
    for (i, (x, reflection)) in [(8., 1.), (32., 1.), (32., -1.), (8., -1.)]
        .into_iter()
        .enumerate()
    {
        if i != 0 {
            tree.update(root, make(x, reflection)).unwrap();
        }
        tree.layout(constraints).unwrap();
        let list = tree.paint();
        let (bounds, mapping) = list
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::PushShaderMask {
                    bounds,
                    mask_transform,
                    ..
                } => Some((bounds, mask_transform)),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            *bounds,
            Rect::from_origin_size(Offset::new(x, 8.), Size::new(16., 16.))
        );
        let edge = |local| {
            if reflection > 0. {
                x + local
            } else {
                x + (16. - local)
            }
        };
        for point in [Offset::new(3., 7.), Offset::new(12., 7.)] {
            assert_eq!(
                mapping.transform_point(point),
                Offset::new(edge(point.x), 15.)
            );
        }
        assert_eq!(calls.get(), 1);
    }
}
