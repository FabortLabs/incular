# Upstream physics dispositions

Ran the unmodified pinned Flutter 3.50.0 spring, friction and gravity test
files. Rust counterparts check the same defining inputs and assertions, with
Rust Duration/Option serving as idiomatic time and unreachable values.
All 12 reference registrations passed. Seven Rust counterparts matched;
five diverged, as recorded in [upstream dispositions](upstream-dispositions.tsv).

| Pinned Flutter test file | Registration lines | Incular results |
| --- | --- | --- |
| spring_simulation_test.dart | 9, 36, 70, 82, 94, 101 | Match: exact snapping, near-critical continuity, positive/negative bounce, getters and a 100 ms period. |
| spring_simulation_test.dart | 114 | Flutter debug rejects zero and negative duration. Incular accepts zero and normalizes to 1 ms by documented design. Negative duration has no Rust Duration representation; not a separate runtime result. |
| friction_simulation_test.dart | 9, 30, 51 | Position/finalX with supplied epsilon, initial velocity, and unreachable controls match. Inverting the returned position misses the default Flutter test tolerance. |
| gravity_simulation_test.dart | 9 | Compact description control passes; Flutter x=500.5, Incular x=500.50003. This exceeds Flutter's default numeric test tolerance. |
| gravity_simulation_test.dart | 14 | Position, velocity and completion on the negative acceleration arc all match. |

Flutter `moreOrLessEquals` defaults to `precisionErrorTolerance` (1e-10).
The Rust inverse errors range from 2.6e-8 to 2.819e-6 seconds (0.026 to
2.819 μs) in the recorded inputs. These are precision differences under
the reference assertions, **not evidence of a user-visible scroll glitch**.
Do not replace exact test expectations with looser ones or claim these
precision differences dominate the actual product impact. Also separate
these from C19 spring classification and C20 termination, which change
categorical behavior. See pinned Flutter
`packages/flutter_test/lib/src/matchers.dart:361–363`,
`foundation/constants.dart:66–71`, and
Incular `crates/incular-animation/src/physics.rs:130–153,627–644`.

## Reproduction

Copy only the three named upstream test files from
`.tmp/flutter/packages/flutter/test/physics/` to the existing scratch
Flutter test package. Run the single spring target and the two friction/gravity
targets. Rust uses temporary `crates/incular-animation/tests/` targets
`audit_upstream_spring_tmp.rs` and `audit_upstream_friction_tmp.rs`;
run `cargo test --locked -p incular-animation --test <target> upstream_ -- --nocapture`.
Logs and scratch sources remain under `.tmp/flutter350-full-audit/`; no
temporary test source remains in the tracked checkout.

```rust
use incular_animation::{Simulation,SpringDescription,SpringSimulation,Tolerance};
use std::time::Duration;
fn ms(n:u64)->Duration{Duration::from_millis(n)}
fn close(x:f32,y:f32,e:f32){assert!((x-y).abs()<=e,"{x} vs {y}, tol {e}");}
#[test]
fn upstream_snap_when_finished(){
    let d=SpringDescription::with_damping_ratio(1.,400.,1.);
    let t=Tolerance{distance:0.1,velocity:0.1,..Tolerance::DEFAULT};
    let s=SpringSimulation::new(d,0.,1.,0.,t);
    assert!(s.position(ms(400))<1.);
    assert!(s.velocity(ms(400))>0.);
    let snap=SpringSimulation::new(d,0.,1.,0.,t).with_snap_to_end(true);
    assert_eq!(snap.position(ms(400)),1.);
    assert_eq!(snap.velocity(ms(400)),0.);
}
#[test]
fn upstream_near_critical_continuity(){
    for r in [1.,1.001,0.999]{
        let s=SpringSimulation::new(SpringDescription::with_damping_ratio(0.4,0.4,r),0.,1.,0.,Tolerance::DEFAULT);
        close(s.position(ms(400)),0.06155,0.01);
        close(s.velocity(ms(400)),0.2681,0.01);
    }
}
fn check_duration_bounce(ms_value:u64,b:f32,k:f32,c:f32,b_epsilon:f32){
    let s=SpringDescription::with_duration_and_bounce(ms(ms_value),b);
    assert_eq!(s.mass,1.);
    close(s.stiffness,k,0.01);
    close(s.damping,c,0.01);
    close(s.bounce(),b,b_epsilon);
    assert_eq!(s.duration(),ms(ms_value));
}
#[test]
fn upstream_duration_bounce_examples(){check_duration_bounce(500,0.3,157.91,17.59,0.0001);}
#[test]
fn upstream_duration_bounce_negative(){check_duration_bounce(500,-0.3,157.91,35.90,0.0001);}
#[test]
fn upstream_duration_bounce_getters(){
    let s=SpringDescription::new(1.,157.91,17.59);
    close(s.bounce(),0.3,0.001);
    assert_eq!(s.duration(),ms(500));
}
#[test]
fn upstream_duration_bounce_custom(){check_duration_bounce(100,0.,3947.84,125.66,0.001);}
#[test]
fn upstream_zero_duration_rejects(){
    let rust=std::panic::catch_unwind(||SpringDescription::with_duration_and_bounce(Duration::ZERO,0.3));
    eprintln!("zero duration result={rust:?}");
    assert!(rust.is_err(),"Flutter rejects zero, Incular synthesizes a 1 ms duration");
}
```

```rust
use incular_animation::{FrictionSimulation,GravitySimulation,Simulation,Tolerance};
use std::time::Duration;
fn ms(n:u64)->Duration{Duration::from_millis(n)}
fn close(x:f32,y:f32,e:f32){assert!((x-y).abs()<=e,"{x} vs {y}, tol {e}");}
fn friction(v:f32,deceleration:f32,wants:[f32;3],final_x:f32,reverse_times:[u64;3],outside:[f32;2]){
    let f=FrictionSimulation::new(0.135,100.,v,Tolerance::DEFAULT).with_constant_deceleration(deceleration);
    assert_eq!(f.position(ms(0)),100.);
    assert_eq!(f.velocity(ms(0)),v);
    for(i,t)in [100,500,2000].into_iter().enumerate(){close(f.position(ms(t)),wants[i],1.);}
    close(f.final_x(),final_x,1.);
    assert_eq!(f.time_at_x(100.),Some(ms(0)));
    assert_eq!(outside.map(|x|f.time_at_x(x)),[None,None]);
    let error=reverse_times.map(|t|{
        let n=f.time_at_x(f.position(ms(t))).unwrap();
        let error=(n.as_secs_f64()-ms(t).as_secs_f64()).abs();
        eprintln!("inverse t={t}ms actual {n:?} abs error={error:e}s");
        error
    });
    assert!(error.iter().all(|&v|v<=1e-10),"inverse target tolerances {error:?}");
}
#[test]fn upstream_positive(){friction(100.,0.,[110.,131.,149.],149.,[100,500,2000],[-1.,200.]);}
#[test]fn upstream_negative(){friction(-100.,0.,[91.,68.,51.],50.,[100,500,2000],[101.,40.]);}
#[test]fn upstream_deceleration(){friction(-100.,100.,[91.,80.,80.],80.,[100,200,300],[101.,40.]);}
#[test]fn upstream_gravity_description_and_position(){
    let g=GravitySimulation::new(9.81,10.,0.,0.);
    assert!(!format!("{g:?}").contains('\n'));
    assert_eq!(g.position(ms(10000)),500.5);
}
#[test]fn upstream_gravity_arc(){
    let g=GravitySimulation::new(-10.,0.,6.,10.);
    for (time,x,velocity) in [(0,0.,10.),(1000,5.,0.),(2000,0.,-10.),(3000,-15.,-20.)] {
        assert_eq!((g.position(ms(time)),g.velocity(ms(time))),(x,velocity));
    }
    assert_eq!([0,200,2000,3000].map(|t|g.is_done(ms(t))),[false,false,false,true]);
}
```

## Upstream clamped simulation controls

Copied the unmodified `clamped_simulation_test.dart` from the pinned
reference, and ran it alone. Both Flutter registrations passed. Both Rust
counterparts passed the same assertions for finite position/velocity bounds
and delegated completion on the gravity baseline. These controls do not
resolve C38–C39 (NaN and inverted bounds).

```rust
use incular_animation::{ClampedSimulation,GravitySimulation,Simulation};
use std::time::Duration;
fn at(ms:u64)->Duration { Duration::from_millis(ms) }
#[test]
fn upstream_clamp_one(){
    let s=ClampedSimulation::new(GravitySimulation::new(9.81,10.,0.,0.),20.,100.)
        .with_velocity_bounds(7.,11.);
    assert_eq!([s.position(at(0)),s.velocity(at(0))],[20.,7.]);
    assert_eq!([s.position(at(100_000)),s.velocity(at(100_000))],[100.,11.]);
}
#[test]
fn upstream_clamp_two(){
    let s=ClampedSimulation::new(GravitySimulation::new(-10.,0.,6.,10.),0.,2.5)
        .with_velocity_bounds(-1.,1.);
    assert_eq!([s.position(at(0)),s.velocity(at(0))],[0.,1.]);
    assert!(!s.is_done(at(0)));
    assert_eq!([s.position(at(1000)),s.velocity(at(1000))],[2.5,0.]);
    assert!(!s.is_done(at(200)));
    assert_eq!([s.position(at(2000)),s.velocity(at(2000))],[0.,-1.]);
    assert!(!s.is_done(at(2000)));
    assert_eq!([s.position(at(3000)),s.velocity(at(3000))],[0.,-1.]);
    assert!(s.is_done(at(3000)));
}
```

Rust used `cargo test --locked -p incular-animation --test audit_upstream_clamped_tmp upstream_clamp -- --nocapture`; temporary target removed.
