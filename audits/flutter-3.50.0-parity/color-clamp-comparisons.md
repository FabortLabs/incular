# Paired slice: hue interpolation and clamped simulations

Flutter 3.50.0 `ab2912035e` vs Incular `c647cb85`. All four
Flutter assertions passed and all four Rust parity assertions failed.

| ID | Contract | Flutter | Incular |
| --- | --- | --- | --- |
| C36 | HSV and HSL midpoint for hue 350° → 10° | [0°, 0°], short arc | [180°, 180°], long arc |
| C37 | HSL from hue 20° to 60° at interpolation factor 1.5 | 80° | 60° |
| C38 | Position and velocity of a clamped simulation when the inner returns NaN | [5, 7], the upper bounds | [NaN, NaN] |
| C39 | Inverted position or velocity bounds in debug | Constructor rejects either | Builder accepts either |

Flutter chooses the short arc and separately clamps interpolated
channels: `painting/colors.dart:30–38,188–203,371–385`. The Rust
`HsvColor` and `HslColor` interpolate hue as a scalar and clamp the
factor: `crates/incular-core/src/geometry.rs:730–754`. The existing
`paint__data__hsv_lerp_hue_wrap` match is a control for its different
inputs, not evidence that this seam-crossing pair matches.

Flutter checks the bounds in the debug constructor and uses framework
`clampDouble` for samples: `physics/clamped_simulation.dart:33–65`.
Rust forwards comparison failures through the clamp helper and has no
matching bounds assertion: `crates/incular-animation/src/physics.rs:62–72,720–753`.
Ordinary finite out-of-bounds inputs matched at [0, 7]. Both
implementations forward the wrapped simulation's unfinished state.
C39 is an input validation difference in debug, not a release guarantee.

## Reproduction

Use the existing pinned scratch Flutter package and run
`flutter test --no-pub test/hue_slice_test.dart` and
`flutter test --no-pub test/clamp_slice_test.dart` separately. For Rust,
use named temporary integration targets, never a workspace run:
`cargo test --locked -p incular-core --test audit_hue_tmp -- --nocapture`
and `cargo test --locked -p incular-animation --test audit_clamp_tmp -- --nocapture`.
Each source target was cleaned after the run. Logs are under
`.tmp/flutter350-full-audit/`.

```dart
import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
void main(){
  test('HSV and HSL choose the short arc across 360',(){
    final hsv=HSVColor.lerp(const HSVColor.fromAHSV(1,350,1,1),
      const HSVColor.fromAHSV(1,10,1,1),0.5)!;
    final hsl=HSLColor.lerp(const HSLColor.fromAHSL(1,350,1,.5),
      const HSLColor.fromAHSL(1,10,1,.5),.5)!;
    expect([hsv.hue,hsl.hue],[0,0]);
  });
  test('HSL interpolates hue past the endpoint',(){
    final hsl=HSLColor.lerp(const HSLColor.fromAHSL(1,20,1,.5),
      const HSLColor.fromAHSL(1,60,1,.5),1.5)!;
    expect(hsl.hue,80);
  });
}
```

```rust
use incular_core::{HslColor,HsvColor,Lerp};
#[test]
fn short_arc_across_360(){
    let hsv=HsvColor::new(350.,1.,1.,1.).lerp(&HsvColor::new(10.,1.,1.,1.),0.5);
    let hsl=HslColor::new(350.,1.,0.5,1.).lerp(&HslColor::new(10.,1.,0.5,1.),0.5);
    assert_eq!([hsv.hue,hsl.hue],[0.,0.]);
}
#[test]
fn hsl_interpolates_hue_past_end(){
    let hsl=HslColor::new(20.,1.,0.5,1.).lerp(&HslColor::new(60.,1.,0.5,1.),1.5);
    assert_eq!(hsl.hue,80.);
}
```

```dart
import 'package:flutter/physics.dart';
import 'package:flutter_test/flutter_test.dart';
class Constant extends Simulation{
  Constant(this.p,this.v);
  final double p,v;
  @override double x(double t)=>p;
  @override double dx(double t)=>v;
  @override bool isDone(double t)=>false;
}
void main(){
  test('clamp NaN maps to upper bound without changing completion',(){
    ClampedSimulation bounded(double x,double v)=>ClampedSimulation(Constant(x,v),xMin:0,xMax:5,dxMin:0,dxMax:7);
    final control=bounded(-1,100);
    expect([control.x(0),control.dx(0)],[0,7]);
    final nan=bounded(double.nan,double.nan);
    expect([nan.x(0),nan.dx(0)],[5,7]);
    expect(nan.isDone(0),isFalse);
  });
  test('inverted clamp bounds are rejected in debug',(){
    expect(()=>ClampedSimulation(Constant(0,0),xMin:5,xMax:0),throwsA(isAssertionError));
    expect(()=>ClampedSimulation(Constant(0,0),dxMin:5,dxMax:0),throwsA(isAssertionError));
  });
}
```

```rust
use incular_animation::{Simulation,ClampedSimulation};
use std::time::Duration;
struct Constant(f32,f32);
impl Simulation for Constant {
    fn position(&self,_:Duration)->f32{self.0}
    fn velocity(&self,_:Duration)->f32{self.1}
    fn is_done(&self,_:Duration)->bool{false}
}
#[test]
fn clamp_nan_maps_to_upper_bound_without_changing_completion(){
    let t=Duration::ZERO;
    let bounded=|x,v|ClampedSimulation::new(Constant(x,v),0.,5.).with_velocity_bounds(0.,7.);
    let normal=bounded(-1.,100.);
    assert_eq!([normal.position(t),normal.velocity(t)],[0.,7.]);
    let nan=bounded(f32::NAN,f32::NAN);
    assert!(!nan.is_done(t));
    assert_eq!([nan.position(t),nan.velocity(t)],[5.,7.]);
}
#[test]
fn inverted_clamp_bounds_are_rejected(){
    let px=std::panic::catch_unwind(||ClampedSimulation::new(Constant(0.,0.),5.,0.)).is_err();
    let dx=std::panic::catch_unwind(||ClampedSimulation::new(Constant(0.,0.),0.,5.).with_velocity_bounds(5.,0.)).is_err();
    assert_eq!([px,dx],[true,true]);
}
```

## Hue interpolation remediation

Pinned `painting/colors_test.dart:168,195,452,479` passed, matching
the native `flutter350_hue_short_arc` suite. Both models cover the
13 original short-arc vectors and four reversible half-turn vectors.
The old native implementation gave 240°, not 330°, at one quarter of
300° → 60°. Both models now cross the correct seam, retain the
direction of a precisely antipodal delta, and permit extrapolation.
Non-hue channels clamp after interpolation rather than clamping the
time. The additional pinned control verifies the channel values and
that an RGB midpoint of 350° → 10° actually paints red. This also
remediates the historical C36 and C37 failures. All 26 selected native
color and API checks passed.
