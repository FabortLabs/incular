//! A sampled transform records its source even when it has no raster area.
use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{Canvas, DisplayList, FilterQuality, ImageSampling, PaintCommand};
use incular_widgets::{
    Center, ClipRect, CustomPaint, CustomPainter, RepaintBoundary, Transform, Widget,
    internal::WidgetTree,
};

struct Counter(Rc<Cell<usize>>);

impl CustomPainter for Counter {
    fn paint(&self, size: Size) -> DisplayList {
        self.0.set(self.0.get() + 1);
        let mut canvas = Canvas::default();
        canvas.rect(Rect::from_origin_size(Offset::ZERO, size), Color::WHITE);
        canvas.finish()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

#[test]
fn singular_sampling_records_a_balanced_filter_stage_with_no_output_area() {
    for ancestor_clip in [false, true] {
        for (x, quality, paints) in [
            (0., None, 0),
            (0., Some(FilterQuality::Low), 1),
            (0.25, Some(FilterQuality::Low), 1),
        ] {
            let count = Rc::new(Cell::new(0));
            let leaf = CustomPaint::from_painter(Size::new(30., 20.), Counter(count.clone()));
            let scale = CoreTransform::scale_non_uniform(x, 1.);
            let transform = Transform::new(scale, RepaintBoundary::new(leaf));
            let transform = quality.map_or_else(
                || transform.clone(),
                |q| transform.clone().filter_quality(q),
            );
            let content = Center::new(transform);
            let mut tree = WidgetTree::new();
            tree.mount(if ancestor_clip {
                ClipRect::new(content).into()
            } else {
                content.into()
            })
            .unwrap();
            tree.layout(Constraints::tight(Size::new(100., 80.)))
                .unwrap();
            let list = tree.try_paint().unwrap();
            assert_eq!(
                count.get(),
                paints,
                "{x}, {quality:?}, clipped={ancestor_clip}"
            );
            let mut entries = list.commands().iter().filter_map(|op| match op {
                PaintCommand::PushMatrixFilter {
                    matrix,
                    bounds,
                    filtered_bounds,
                    sampling,
                    ..
                } => Some((*matrix, *bounds, *filtered_bounds, *sampling)),
                _ => None,
            });
            if quality.is_some() {
                let (matrix, source, filtered, sampling) = entries.next().expect("filter recorded");
                assert_eq!(matrix, scale);
                assert_eq!(source.size, Size::new(30., 20.));
                assert_eq!(sampling, ImageSampling::Linear);
                if x == 0. {
                    assert_eq!(filtered.size.width, 0., "{filtered:?}");
                } else {
                    assert_eq!(filtered.size, Size::new(7.5, 20.));
                }
                assert!(
                    list.commands()
                        .iter()
                        .any(|op| matches!(op, PaintCommand::PopEffect))
                );
            }
            assert!(entries.next().is_none());
        }
    }
}

#[test]
fn changing_only_filter_quality_at_zero_updates_cached_clip_culling() {
    let count = Rc::new(Cell::new(0));
    let retained = Widget::from(RepaintBoundary::new(CustomPaint::from_painter(
        Size::new(30., 20.),
        Counter(count.clone()),
    )));
    let view = |x, y, filtered| {
        let transform = Transform::new(CoreTransform::scale_non_uniform(x, y), retained.clone());
        ClipRect::new(Center::new(if filtered {
            transform.filter_quality(FilterQuality::Low)
        } else {
            transform
        }))
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(view(0., 1., false)).unwrap();
    for (x, y, filtered, total_paints) in [
        (0., 1., false, 0),
        (0., 1., true, 1),
        (1., 0., false, 1),
        (1., 0., true, 1),
    ] {
        tree.update(root, view(x, y, filtered)).unwrap();
        tree.layout(Constraints::tight(Size::new(100., 80.)))
            .unwrap();
        tree.update_compositor(incular_core::time::Instant::now())
            .unwrap();
        let list = tree.try_paint().unwrap();
        assert_eq!(count.get(), total_paints);
        assert_eq!(
            list.commands()
                .iter()
                .any(|op| matches!(op, PaintCommand::PushMatrixFilter { .. })),
            filtered,
        );
    }
}
