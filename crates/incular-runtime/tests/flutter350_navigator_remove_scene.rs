use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_navigation::{Navigator, Route, RouteTransition};
use incular_rendering::{Brush, DisplayList, PaintCommand};
use incular_runtime::{RouteOutlet, Runtime};
use incular_widgets::{Stack, Widget};

const HOME: Color = Color::rgba(41, 7, 15, 255);
const A: Color = Color::rgba(8, 60, 9, 255);
const B: Color = Color::rgba(18, 29, 71, 255);

fn page(color: Color) -> Widget {
    Widget::box_(Size::new(200., 150.), color)
}

fn paints(list: &DisplayList, color: Color) -> bool {
    list.commands().iter().any(|command| match command {
        PaintCommand::Rect { color: fill, .. }
        | PaintCommand::RRect {
            brush: Brush::Solid(fill),
            ..
        }
        | PaintCommand::FillPath {
            brush: Brush::Solid(fill),
            ..
        } => *fill == color,
        _ => false,
    })
}

#[test]
fn removing_the_current_page_reveals_its_predecessor_without_reverse_paint() {
    let navigator = Navigator::new();
    let start = Instant::now();
    let root = navigator.push(Route::new("/", page(HOME)));
    let mut runtime = Runtime::new(Stack::new(Vec::<Widget>::new()).into()).unwrap();
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(
        &navigator,
        &runtime.spawner().scope(),
    )));
    let mount = runtime.tree().root().unwrap();
    runtime
        .register_builder(mount, {
            let outlet = outlet.clone();
            move || Stack::new([outlet.borrow().widget()]).into()
        })
        .unwrap();
    let limits = Constraints::tight(Size::new(200., 150.));
    runtime.run_frame_at(limits, start).unwrap();
    RouteOutlet::attach(&outlet, &mut runtime).unwrap();
    let present = |runtime: &mut Runtime, frame: u64| {
        RouteOutlet::present_frame_at(
            &outlet,
            runtime,
            limits,
            start + Duration::from_millis(16 * frame),
        )
        .unwrap()
        .0
    };

    let initial = present(&mut runtime, 0);
    assert!(paints(&initial, HOME));
    let alpha_transition = RouteTransition::fade_in(Duration::from_millis(300), start);
    let a_animation = alpha_transition.animation().unwrap();
    let alpha = navigator.push(Route::new("/A", page(A)).transition(alpha_transition));
    for step in 1..25 {
        present(&mut runtime, step);
    }
    assert!(a_animation.is_completed());

    let beta_transition = RouteTransition::fade_in(
        Duration::from_millis(300),
        start + Duration::from_millis(25 * 16),
    );
    let b_animation = beta_transition.animation().unwrap();
    let beta = navigator.push(Route::new("/B", page(B)).transition(beta_transition));
    for step in 25..50 {
        present(&mut runtime, step);
    }
    let before = present(&mut runtime, 50);
    assert!(
        paints(&before, B),
        "before removal: {:?}",
        before.commands()
    );
    assert!(b_animation.is_completed());

    let removed = navigator.remove_route(beta).unwrap();
    assert_eq!(removed.id, beta);
    let after_b = present(&mut runtime, 51);
    assert!(
        !paints(&after_b, B),
        "no outgoing B paint in the first frame"
    );
    assert!(paints(&after_b, A));
    assert_eq!(navigator.current().unwrap().id, alpha);
    assert!(!b_animation.is_animating());

    let removed = navigator.remove_route(alpha).unwrap();
    assert_eq!(removed.id, alpha);
    let after_a = present(&mut runtime, 52);
    assert!(
        !paints(&after_a, A),
        "no outgoing A paint in the first frame"
    );
    assert!(paints(&after_a, HOME));
    assert_eq!(navigator.current().unwrap().id, root);
    assert!(!a_animation.is_animating());
}
