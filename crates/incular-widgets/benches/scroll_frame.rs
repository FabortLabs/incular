//! One frame of scrolling a long lazy list: the offset moves, layout
//! materializes the rows that enter the cache window, the compositor follows
//! the new offset and the scene is painted.
use criterion::{Criterion, criterion_group, criterion_main};
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::{ScrollController, Sliver, WidgetTree};
use incular_widgets::{
    CustomScrollView, Directionality, Padding, SliverList, Text, TextDirection, Widget,
};
use std::time::Instant;

const ITEMS: usize = 100_000;

fn list_tree(viewport: Size) -> (WidgetTree, ScrollController) {
    let controller = ScrollController::new();
    let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(SliverList::builder(ITEMS, |index| {
        Widget::from(Padding::all(
            8.,
            Text::new(format!("a moderately long row label number {index}")),
        ))
    }))];
    let list: Widget = CustomScrollView::new(slivers)
        .controller(controller.clone())
        .into();
    let mut tree = WidgetTree::default();
    tree.mount(Directionality::new(TextDirection::Ltr, list).into())
        .unwrap();
    frame(&mut tree, viewport);
    (tree, controller)
}

fn frame(tree: &mut WidgetTree, viewport: Size) {
    tree.layout(Constraints::tight(viewport)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    std::hint::black_box(tree.paint());
}

fn bench(c: &mut Criterion) {
    let viewport = Size::new(600., 800.);
    let mut group = c.benchmark_group("scroll_frame");
    // A few pixels move the retained rows; a row height or more also
    // materializes new rows; a page replaces every visible row.
    for (name, step) in [("step_3px", 3.), ("step_40px", 40.), ("step_page", 800.)] {
        let (mut tree, controller) = list_tree(viewport);
        let limit = controller.max_offset().max(1.) * 0.5;
        let mut offset = 0_f32;
        let mut direction = 1_f32;
        group.bench_function(name, |b| {
            b.iter(|| {
                // Sweep back and forth so the list never runs out.
                if offset + direction * step > limit || offset + direction * step < 0. {
                    direction = -direction;
                }
                offset += direction * step;
                controller.jump_to(offset);
                frame(&mut tree, viewport);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
