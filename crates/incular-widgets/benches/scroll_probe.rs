//! Temporary phase probe (not registered for commit).
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::{ScrollController, Sliver, WidgetTree};
use incular_widgets::{CustomScrollView, Directionality, Padding, SliverList, Text, TextDirection, Widget};
use std::time::{Duration, Instant};

fn main() {
    let viewport = Size::new(600., 800.);
    for step in [3_f32, 40., 800.] {
        let controller = ScrollController::new();
        let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(SliverList::builder(100_000, |index| {
            Widget::from(Padding::all(8., Text::new(format!("a moderately long row label number {index}"))))
        }))];
        let list: Widget = CustomScrollView::new(slivers).controller(controller.clone()).into();
        let mut tree = WidgetTree::default();
        tree.mount(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
        tree.layout(Constraints::tight(viewport)).unwrap();
        tree.update_compositor(Instant::now()).unwrap();
        let _ = tree.paint();
        let (mut layout, mut composite, mut paint) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        let frames = 3000;
        let mut offset = 0_f32;
        let before = tree.diagnostics();
        for _ in 0..frames {
            offset += step;
            controller.jump_to(offset);
            let t = Instant::now();
            tree.layout(Constraints::tight(viewport)).unwrap();
            layout += t.elapsed();
            let t = Instant::now();
            tree.update_compositor(Instant::now()).unwrap();
            composite += t.elapsed();
            let t = Instant::now();
            std::hint::black_box(tree.paint());
            paint += t.elapsed();
        }
        let after = tree.diagnostics();
        println!(
            "step {step}: layout {:?} composite {:?} paint {:?} per frame",
            layout / frames, composite / frames, paint / frames
        );
        let _ = (&before, &after);
        incular_widgets::probe::report(frames as u64);
    }
}
