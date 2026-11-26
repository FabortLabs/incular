use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::{Alignment, Constraints};
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Border, BorderRadius, BorderSide, BoxDecoration, BoxShape, Container, GestureDetector,
    HitTestBehavior, Radius,
};

fn check(
    width: f32,
    height: f32,
    alignment: Alignment,
    decoration: BoxDecoration,
    points: &[(Offset, bool)],
) {
    let count = Rc::new(Cell::new(0));
    let border = Border::all(BorderSide::solid(Color::BLACK, 1.));
    let widget = Align::new(
        alignment,
        GestureDetector::new(
            Container::new()
                .width(width)
                .height(height)
                .decoration(decoration.border(border)),
        )
        .behavior(HitTestBehavior::DeferToChild)
        .on_tap({
            let count = count.clone();
            move || count.set(count.get() + 1)
        }),
    );
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let start = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), start)
        .unwrap();
    assert_eq!(count.get(), 0);
    for (index, &(position, hit)) in points.iter().enumerate() {
        let previous = count.get();
        let at = start + Duration::from_millis(index as u64 * 400 + 1);
        runtime.handle_input_at(
            InputEvent::Pointer {
                phase: PointerPhase::Down,
                position,
            },
            at,
        );
        runtime.handle_input_at(
            InputEvent::Pointer {
                phase: PointerPhase::Up,
                position,
            },
            at + Duration::from_millis(1),
        );
        assert_eq!(
            count.get(),
            previous + usize::from(hit),
            "tap at {position:?}"
        );
    }
}

#[test]
fn a_border_only_rect_accepts_the_center_and_inside_corners() {
    check(
        100.,
        50.,
        Alignment::CENTER,
        BoxDecoration::new(),
        &[
            (Offset::new(400., 300.), true),
            (Offset::new(350., 275.), true),
            (Offset::new(449., 324.), true),
        ],
    );
}

#[test]
fn a_circle_checks_its_shape_not_the_enclosing_wide_box() {
    check(
        100.,
        50.,
        Alignment::CENTER,
        BoxDecoration::new().shape(BoxShape::Circle),
        &[
            (Offset::ZERO, false),
            (Offset::new(350., 275.), false),
            (Offset::new(400., 300.), true),
            (Offset::new(400., 300.), true),
        ],
    );
}

#[test]
fn a_rounded_border_hits_the_rounded_geometry() {
    check(
        100.,
        50.,
        Alignment::CENTER,
        rounded(20.),
        &[
            (Offset::ZERO, false),
            (Offset::new(350., 275.), false),
            (Offset::new(400., 300.), true),
            (Offset::new(400., 300.), true),
        ],
    );
}

fn rounded(radius: f32) -> BoxDecoration {
    BoxDecoration::new().border_radius(BorderRadius::all(Radius::circular(radius)))
}

#[test]
fn the_rounded_top_left_rejects_all_four_outside_samples() {
    let radius = 12.3_f64;
    let diagonal = (radius - radius * std::f64::consts::FRAC_1_SQRT_2) as f32;
    check(
        50.,
        50.,
        Alignment::TOP_LEFT,
        rounded(radius as f32),
        &[
            (Offset::ZERO, false),
            (Offset::new(radius as f32 - 1., 0.), false),
            (Offset::new(0., radius as f32 - 1.), false),
            (Offset::new(diagonal, diagonal), false),
            (Offset::new(25., 25.), true),
        ],
    );
}

#[test]
fn the_rounded_top_left_accepts_tangencies_and_the_inside_sample() {
    let radius = 12.3_f64;
    let inside_diagonal = (radius - radius * std::f64::consts::FRAC_1_SQRT_2 + 1.) as f32;
    check(
        50.,
        50.,
        Alignment::TOP_LEFT,
        rounded(radius as f32),
        &[
            (Offset::new(radius as f32, 0.), true),
            (Offset::new(0., radius as f32), true),
            (Offset::new(inside_diagonal, inside_diagonal), true),
            (Offset::new(25., 25.), true),
        ],
    );
}
