use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::Runtime;
use incular_widgets::Widget;

const BASE: Color = Color::rgba(255, 144, 0, 255);
const GREEN: Color = Color::rgba(15, 249, 0, 255);

fn mount() -> Runtime {
    Runtime::new(Widget::box_(Size::new(10., 10.), BASE)).unwrap()
}

#[test]
fn render_view_paint_bounds_include_device_ratio_without_changing_logical_layout() {
    let mut runtime = mount();
    for (width, height, dpr) in [(800, 600, 2.5), (640, 400, 2.), (256, 97, 1.)] {
        let metrics = WindowMetrics::new(PhysicalSize::new(width, height), dpr);
        runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
        runtime
            .run_frame(Constraints::tight(metrics.logical_size()))
            .unwrap();
        assert_eq!(
            runtime.tree().root_layout_size(),
            Some(metrics.logical_size())
        );
        assert_eq!(
            runtime.view_paint_bounds(),
            Some(Rect::from_origin_size(
                Offset::ZERO,
                Size::new(width as f32, height as f32)
            )),
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn debug_view_painters_draw_after_content_and_can_be_removed_without_requesting_a_frame() {
    use incular_rendering::{DisplayList, PaintCommand};
    use std::{cell::RefCell, rc::Rc};

    let color_order = |display: &DisplayList| {
        display
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Rect { color, .. } => Some(*color),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let mut runtime = mount();
    let size = Size::new(100., 200.);
    let constraints = Constraints::tight(size);
    let (before, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(color_order(&before), [BASE]);
    let frame_requested = runtime.frame_requested();

    // The first callback may remove another callback before its turn.
    let removed = Rc::new(RefCell::new(None));
    let first = runtime
        .debug_add_paint_callback({
            let removed = Rc::clone(&removed);
            move |display, logical_size| {
                assert_eq!(logical_size, size);
                removed.borrow_mut().take();
                display.push(PaintCommand::Rect {
                    rect: Rect::from_origin_size(Offset::new(2., 2.), Size::new(8., 8.)),
                    color: GREEN,
                });
            }
        })
        .unwrap();
    *removed.borrow_mut() = runtime.debug_add_paint_callback(|_, _| panic!("removed before paint"));
    assert_eq!(runtime.frame_requested(), frame_requested);
    let (with_overlay, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(color_order(&with_overlay), [BASE, GREEN]);

    drop(first);
    assert_eq!(runtime.frame_requested(), frame_requested);
    let (after, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(color_order(&after), [BASE]);
    assert_eq!(color_order(&before), [BASE]);
}

#[test]
fn prelayout_metrics_replacements_use_the_latest_configuration() {
    let mut runtime = mount();
    for (width, height, ratio) in [(300, 600, 3.), (400, 600, 2.)] {
        runtime.handle_platform_event(PlatformEvent::Metrics(WindowMetrics::new(
            PhysicalSize::new(width, height),
            ratio,
        )));
    }
    let view = runtime.environment();
    runtime
        .run_frame(Constraints::tight(view.viewport))
        .unwrap();
    assert_eq!(
        runtime.tree().root_layout_size(),
        Some(Size::new(200., 300.))
    );
    assert_eq!(
        runtime.view_paint_bounds(),
        Some(Rect::from_origin_size(Offset::ZERO, Size::new(400., 600.))),
    );
}

#[test]
fn retained_root_layer_survives_a_resize_and_unchanged_view_metrics() {
    let mut runtime = mount();
    let mut retained = None;
    for (width, height, ratio) in [
        (200, 200, 2.),
        (200, 200, 2.),
        (200, 2234, 2.),
        // The native renderer applies DPR at the surface; unlike Flutter's
        // RenderView it need not replace the logical root to change scale.
        (200, 2234, 5.),
    ] {
        let metrics = WindowMetrics::new(PhysicalSize::new(width, height), ratio);
        runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
        runtime
            .run_frame(Constraints::tight(metrics.logical_size()))
            .unwrap();
        let root = runtime.tree().retained_root_layer().unwrap();
        assert_eq!(root, *retained.get_or_insert(root));
        assert_eq!(
            runtime.tree().root_layout_size(),
            Some(metrics.logical_size())
        );
    }
}
