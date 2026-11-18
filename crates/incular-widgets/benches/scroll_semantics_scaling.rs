//! Scroll semantics beside retained content and with genuinely moving content.
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_semantics::SemanticRole;
use incular_widgets::internal::{ExplicitSemantics, WidgetTree};
use incular_widgets::{Column, Row, ScrollController, SingleChildScrollView, SizedBox, Widget};

fn semantic_box(label: impl Into<String>, height: f32) -> Widget {
    Widget::box_(Size::new(100., height), Color::WHITE)
        .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label(label))
}

fn semantic_column(count: usize) -> Widget {
    Widget::from(Column::new(
        (0..count).map(|index| semantic_box(format!("item {index}"), 2.)),
    ))
    .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label("retained group"))
}

fn viewport(controller: &ScrollController, child: Widget) -> Widget {
    SizedBox::from_dimensions(
        Some(100.),
        Some(100.),
        Some(SingleChildScrollView::new(child).controller(controller.clone())),
    )
    .into()
}

fn prepared(widget: Widget, size: Size) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount benchmark tree");
    tree.layout(Constraints::tight(size)).expect("layout");
    tree.update_compositor(Instant::now()).expect("compositor");
    let _ = tree.paint();
    tree.update_semantics();
    tree
}

fn scrolling_frame(tree: &mut WidgetTree, controllers: &[ScrollController], offset: f32) {
    for controller in controllers {
        controller.jump_to(offset);
    }
    tree.update_compositor(Instant::now()).expect("compositor");
    tree.update_semantics();
    std::hint::black_box(tree.semantics().revision());
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("scroll_semantics");
    for count in [100_usize, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("static_siblings", count),
            &count,
            |b, &count| {
                let controller = ScrollController::new();
                let root = Widget::from(
                    Row::new([
                        viewport(&controller, semantic_box("moving", 800.)),
                        semantic_column(count),
                    ])
                    .text_direction(TextDirection::Ltr),
                )
                .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label("root"));
                let mut tree = prepared(root, Size::new(200., 100.));
                assert!(tree.semantics().len() >= count);
                let mut offset = 20.;
                b.iter(|| {
                    offset = if offset == 20. { 40. } else { 20. };
                    scrolling_frame(&mut tree, std::slice::from_ref(&controller), offset);
                });
            },
        );

        // Control: every semantic descendant really moves, so collecting
        // its geometry must still scale with the scrollable's contents.
        group.bench_with_input(
            BenchmarkId::new("moving_descendants", count),
            &count,
            |b, &count| {
                let controller = ScrollController::new();
                let mut tree = prepared(
                    viewport(&controller, semantic_column(count)),
                    Size::new(100., 100.),
                );
                let mut offset = 20.;
                b.iter(|| {
                    offset = if offset == 20. { 40. } else { 20. };
                    scrolling_frame(&mut tree, std::slice::from_ref(&controller), offset);
                });
            },
        );
    }

    // Control: many viewports all change. No unrelated semantic owners
    // can be skipped, and every viewport must retain fresh edge actions.
    for count in [1_usize, 128] {
        group.bench_with_input(
            BenchmarkId::new("all_viewports", count),
            &count,
            |b, &count| {
                let controllers: Vec<_> = (0..count).map(|_| ScrollController::new()).collect();
                let root =
                    Column::new(controllers.iter().enumerate().map(|(index, controller)| {
                        viewport(controller, semantic_box(format!("moving {index}"), 800.))
                    }));
                let mut tree = prepared(root.into(), Size::new(100., 100. * count as f32));
                let mut offset = 20.;
                b.iter(|| {
                    offset = if offset == 20. { 40. } else { 20. };
                    scrolling_frame(&mut tree, &controllers, offset);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
