use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use incular_animation::{
    BoundedFrictionSimulation, ClampedSimulation, FrictionSimulation, GravitySimulation,
    ScrollSpringSimulation, Simulation, SpringDescription, SpringSimulation, SpringType, Tolerance,
};
use incular_parity_harness::{FrameHarness, Scenario};
use incular_scroll::ScrollPhysics;
use incular_widgets::SizedBox;

const FRAME: Duration = Duration::from_millis(16);

fn clock() -> Rc<Cell<u64>> {
    Rc::new(Cell::new(0))
}

fn pump(h: &mut FrameHarness, clock: &Cell<u64>, duration: Duration) {
    clock.set(clock.get() + duration.as_micros() as u64);
    h.pump(duration);
}

fn frames(h: &mut FrameHarness, clock: &Cell<u64>, count: usize) {
    for _ in 0..count {
        pump(h, clock, FRAME);
    }
}

fn record_simulation(
    h: &mut FrameHarness,
    name: &str,
    simulation: Rc<dyn Simulation>,
    clock: Rc<Cell<u64>>,
) {
    h.value(&format!("{name}.x"), {
        let simulation = Rc::clone(&simulation);
        let clock = Rc::clone(&clock);
        move || f64::from(simulation.position(Duration::from_micros(clock.get())))
    });
    h.value(&format!("{name}.dx"), {
        let simulation = Rc::clone(&simulation);
        let clock = Rc::clone(&clock);
        move || f64::from(simulation.velocity(Duration::from_micros(clock.get())))
    });
    h.value(&format!("{name}.done"), {
        let simulation = Rc::clone(&simulation);
        move || {
            if simulation.is_done(Duration::from_micros(clock.get())) {
                1.
            } else {
                0.
            }
        }
    });
}

fn spring_type_value(spring_type: SpringType) -> f64 {
    match spring_type {
        SpringType::CriticallyDamped => 0.,
        SpringType::UnderDamped => 1.,
        SpringType::OverDamped => 2.,
    }
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("physics__tolerance__default", tolerance_default),
        (
            "physics__spring_description__damping_regimes",
            spring_description_damping_regimes,
        ),
        (
            "physics__spring_description__duration_bounce",
            spring_description_duration_bounce,
        ),
        ("physics__spring__damping_regimes", spring_damping_regimes),
        (
            "physics__spring__tolerance_boundary",
            spring_tolerance_boundary,
        ),
        ("physics__friction__ordinary", friction_ordinary),
        (
            "physics__friction__tolerance_boundary",
            friction_tolerance_boundary,
        ),
        (
            "physics__bounded_friction__position_and_velocity",
            bounded_friction_position_and_velocity,
        ),
        (
            "physics__bounded_friction__distance_tolerance",
            bounded_friction_distance_tolerance,
        ),
        (
            "physics__gravity__negative_acceleration_threshold",
            gravity_negative_acceleration_threshold,
        ),
        (
            "physics__clamped_simulation__position_and_velocity",
            clamped_simulation_position_and_velocity,
        ),
        (
            "physics__bouncing_scroll__extent_handoff",
            bouncing_scroll_extent_handoff,
        ),
        (
            "physics__scroll_spring__snap_when_done",
            scroll_spring_snap_when_done,
        ),
        ("physics__clamping_scroll__spline", clamping_scroll_spline),
    ]
}

fn tolerance_default(h: &mut FrameHarness) {
    let tolerance = Tolerance::DEFAULT;
    h.value("distance", move || f64::from(tolerance.distance));
    h.value("time", move || tolerance.time.as_secs_f64());
    h.value("velocity", move || f64::from(tolerance.velocity));
    h.mount(|_| SizedBox::shrink().into());
    h.frames(3);
}

fn spring_description_damping_regimes(h: &mut FrameHarness) {
    let under = SpringDescription::with_damping_ratio(1., 100., 0.5);
    let critical = SpringDescription::with_damping_ratio(1., 100., 1.);
    let over = SpringDescription::with_damping_ratio(1., 100., 2.);
    let negative_ratio = SpringDescription::with_damping_ratio(1., 100., -0.25);
    let raw = SpringDescription::new(0., 0., -1.);
    h.value("under.damping", move || f64::from(under.damping));
    h.value("under.type", move || spring_type_value(under.spring_type()));
    h.value("critical.damping", move || f64::from(critical.damping));
    h.value("critical.type", move || {
        spring_type_value(critical.spring_type())
    });
    h.value("over.damping", move || f64::from(over.damping));
    h.value("over.type", move || spring_type_value(over.spring_type()));
    h.value("negative_ratio.damping", move || {
        f64::from(negative_ratio.damping)
    });
    h.value("raw.mass", move || f64::from(raw.mass));
    h.value("raw.stiffness", move || f64::from(raw.stiffness));
    h.value("raw.damping", move || f64::from(raw.damping));
    h.value("negative_ratio.type", move || {
        spring_type_value(negative_ratio.spring_type())
    });
    h.mount(|_| SizedBox::shrink().into());
    h.frames(2);
}

fn spring_description_duration_bounce(h: &mut FrameHarness) {
    let spring = SpringDescription::with_duration_and_bounce(Duration::from_micros(300_500), 0.3);
    let ratio = spring.damping / (2. * (spring.mass * spring.stiffness).sqrt());
    let duration_ms =
        (4. * std::f32::consts::PI.powi(2) * spring.mass / spring.stiffness).sqrt() * 1_000.;
    let bounce = if ratio < 1. {
        1. - ratio
    } else {
        1. / ratio - 1.
    };
    h.value("mass", move || f64::from(spring.mass));
    h.value("stiffness", move || f64::from(spring.stiffness));
    h.value("damping", move || f64::from(spring.damping));
    h.value("duration_ms", move || f64::from(duration_ms.round()));
    h.value("bounce", move || f64::from(bounce));
    h.mount(|_| SizedBox::shrink().into());
    h.frames(2);
}

fn spring_damping_regimes(h: &mut FrameHarness) {
    let clock = clock();
    let regimes = [
        (
            "under",
            SpringDescription::with_damping_ratio(1., 100., 0.5),
        ),
        (
            "critical",
            SpringDescription::with_damping_ratio(1., 100., 1.),
        ),
        ("over", SpringDescription::with_damping_ratio(1., 100., 2.)),
    ];
    for (name, description) in regimes {
        record_simulation(
            h,
            name,
            Rc::new(SpringSimulation::new(
                description,
                0.,
                100.,
                30.,
                Tolerance::DEFAULT,
            )),
            Rc::clone(&clock),
        );
        h.value(&format!("{name}.type"), move || {
            spring_type_value(description.spring_type())
        });
    }
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 40);
}

fn spring_tolerance_boundary(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "spring",
        Rc::new(SpringSimulation::new(
            SpringDescription::with_damping_ratio(1., 100., 1.),
            0.001,
            0.,
            0.,
            Tolerance::DEFAULT,
        )),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 2);
}

fn friction_ordinary(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "friction",
        Rc::new(FrictionSimulation::new(
            0.135,
            10.,
            120.,
            Tolerance::DEFAULT,
        )),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 40);
}

fn friction_tolerance_boundary(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "friction",
        Rc::new(FrictionSimulation::new(
            0.135,
            0.,
            0.001,
            Tolerance::new(0.001, Duration::from_millis(1), 0.001),
        )),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 2);
}

fn bounded_friction_position_and_velocity(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "bounded",
        Rc::new(BoundedFrictionSimulation::new(0.135, 0., 100., -2., 4.)),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 12);
}

fn bounded_friction_distance_tolerance(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "bounded",
        Rc::new(BoundedFrictionSimulation::new(0.135, 3.9995, 100., -2., 4.)),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 2);
}

fn gravity_negative_acceleration_threshold(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "gravity",
        Rc::new(GravitySimulation::new(-10., 0., 100., 0.)),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 30);
}

fn clamped_simulation_position_and_velocity(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "clamped",
        Rc::new(ClampedSimulation::new(
            GravitySimulation::new(100., 0., 50., 200.),
            0.,
            20.,
        )),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 30);
}

fn bouncing_scroll_extent_handoff(h: &mut FrameHarness) {
    let clock = clock();
    let simulation = ScrollPhysics::clamping()
        .bouncing()
        .ballistic_simulation(90., 3000., 0., 100.)
        .expect("fling crosses the trailing extent");
    h.value("x", {
        let simulation_for_x = simulation;
        let clock = Rc::clone(&clock);
        move || f64::from(simulation_for_x.x(clock.get() as f32 / 1_000_000.))
    });
    h.value("dx", {
        let simulation_for_dx = simulation;
        let clock = Rc::clone(&clock);
        move || f64::from(simulation_for_dx.dx(clock.get() as f32 / 1_000_000.))
    });
    h.value("done", {
        let simulation_for_done = simulation;
        let clock = Rc::clone(&clock);
        move || {
            if simulation_for_done.is_done(clock.get() as f32 / 1_000_000.) {
                1.
            } else {
                0.
            }
        }
    });
    h.mount(|_| SizedBox::shrink().into());
    pump(h, &clock, Duration::from_millis(1));
    frames(h, &clock, 120);
}

fn scroll_spring_snap_when_done(h: &mut FrameHarness) {
    let clock = clock();
    record_simulation(
        h,
        "scroll_spring",
        Rc::new(ScrollSpringSimulation::new(
            SpringDescription::with_damping_ratio(1., 100., 1.5),
            0.,
            100.,
            0.,
            Tolerance::DEFAULT,
        )),
        Rc::clone(&clock),
    );
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 100);
}

fn clamping_scroll_spline(h: &mut FrameHarness) {
    let clock = clock();
    let simulation = ScrollPhysics::clamping()
        .ballistic_simulation(70., 500., 0., 320.)
        .expect("fast enough");
    h.value("x", {
        let simulation_for_x = simulation;
        let clock = Rc::clone(&clock);
        move || f64::from(simulation_for_x.x(clock.get() as f32 / 1_000_000.))
    });
    h.value("dx", {
        let simulation_for_dx = simulation;
        let clock = Rc::clone(&clock);
        move || f64::from(simulation_for_dx.dx(clock.get() as f32 / 1_000_000.))
    });
    h.value("done", {
        let simulation_for_done = simulation;
        let clock = Rc::clone(&clock);
        move || {
            if simulation_for_done.is_done(clock.get() as f32 / 1_000_000.) {
                1.
            } else {
                0.
            }
        }
    });
    h.mount(|_| SizedBox::shrink().into());
    frames(h, &clock, 20);
}
