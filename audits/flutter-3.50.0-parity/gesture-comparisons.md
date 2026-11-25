# Paired slice: gesture arbitration and velocity

Flutter 3.50.0 at `ab2912035e` and Incular implementation `c647cb85`;
audit HEAD before probes `388deee7`. These tests inject pointer sequences,
not actual touch hardware.

| ID | Scenario | Flutter | Incular |
| --- | --- | --- | --- |
| C09 | Inner horizontal drag rotated 90°, competing outer tap; move 20 px along the global vertical axis | Starts the local horizontal drag once | Does not start it |
| C09 controls | Unrotated horizontal 20 px, then a separate rotated sequence at 17 px | 1 start and 0 starts, respectively | 1 start and 0 starts, respectively |
| C10 | Four synthesized touch moves at 20 px per 10 ms in a scale gesture | End velocity 0 px/s | End velocity 2,000 px/s |
| C10 control | Same path with ordinary moves | End velocity above 1,000 px/s | End velocity above 1,000 px/s |

Flutter projects local deltas for axis arbitration, then evaluates physical
travel (`gestures/monodrag.dart:672–719,1029–1040`). Incular supplies local
details but recognizes using global axis travel in the threshold accumulator
(`crates/incular-gestures/src/pointer.rs:513–537,636–651,840–853`).
Flutter updates scale geometry but excludes synthesized samples from velocity
history (`gestures/scale.dart:545–555,694–722`); Incular adds all Move samples
(`crates/incular-gestures/src/scale.rs:309–316,480–501`). Do not generalize
this mismatch to MultiDrag, which has a separate synthesized-event filter.

Next nearby cases: opposite rotation, scale-plus-rotation, exact threshold
and a longer/multiple-device route; for scale, mixed genuine/synthesized
samples and native trackpad streams are still open. These were custom probes,
not closure of the catalogued upstream registrations.

## Focused reproduction

Use the pinned Flutter scratch package previously documented in
[focused comparisons](focused-comparisons.md), with this as
`test/gesture_slice_test.dart`. Run only
`flutter test --no-pub test/gesture_slice_test.dart`. Both tests passed.
Use the temporary Rust targets below and remove the copies after running.
Both Incular parity assertions failed as listed; the positive controls ran
before those assertions. Logs are under `.tmp/flutter350-full-audit/`.

```dart
import 'dart:math' as math;
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main(){
  testWidgets('rotated local horizontal drag competes in the correct axis',(t) async{
    Future<int> run(double angle,Offset delta) async {
      const key=ValueKey('target');
      var starts=0;
      await t.pumpWidget(Directionality(textDirection:TextDirection.ltr,
        child:Center(child:GestureDetector(onTap:(){}, child:Transform.rotate(angle:angle,
          child:GestureDetector(behavior:HitTestBehavior.opaque,
            onHorizontalDragStart:(_){starts++;},child:const SizedBox(key:key,width:200,height:200)))))));
      final p=t.getTopLeft(find.byKey(key))+ (angle==0?const Offset(50,50):const Offset(-50,50));
      final g=await t.startGesture(p);
      await g.moveBy(delta);
      final n=starts;
      await g.cancel();
      return n;
    }
    expect(await run(0,const Offset(20,0)),1);
    expect(await run(math.pi/2,const Offset(0,17)),0);
    expect(await run(math.pi/2,const Offset(0,20)),1);
  });
  testWidgets('synthesized moves do not feed scale velocity',(t) async {
    Future<double> run(bool synthetic,int pointer) async{
      final ends=<double>[];
      await t.pumpWidget(Directionality(textDirection:TextDirection.ltr,
        child:GestureDetector(behavior:HitTestBehavior.opaque,
          onScaleStart:(_){},onScaleEnd:(d)=>ends.add(d.velocity.pixelsPerSecond.dx),
          child:const SizedBox.expand())));
      const start=Offset(100,100);
      t.binding.handlePointerEvent(PointerDownEvent(pointer:pointer,position:start));
      for(var k=1;k<=4;k++){
        t.binding.handlePointerEvent(PointerMoveEvent(pointer:pointer,position:start+Offset(20.0*k,0),
          delta:const Offset(20,0),timeStamp:Duration(milliseconds:10*k),synthesized:synthetic));
      }
      t.binding.handlePointerEvent(PointerUpEvent(pointer:pointer,position:start+const Offset(80,0),
        timeStamp:const Duration(milliseconds:45)));
      return ends.single;
    }
    expect(await run(false,1),greaterThan(1000));
    expect(await run(true,2),0);
  });
}
```

Append to a temporary copy of
`crates/incular-widgets/tests/gesture_local_positions.rs`, as
`audit_rotated_gesture_tmp.rs`. Run
`cargo test --locked -p incular-widgets --test audit_rotated_gesture_tmp audit_gesture_`.

```rust
#[test]
fn audit_gesture_rotated_local_horizontal_competes_correctly() {
    let run=|angle:f32, dx:f32, dy:f32| {
        let starts=Rc::new(std::cell::Cell::new(0));
        let seen=starts.clone();
        let inner=GestureDetector::new(target()).on_horizontal_drag_start(move |_|seen.set(seen.get()+1));
        let outer=GestureDetector::new(Transform::rotation(angle,inner)).on_tap(||{});
        let mut tree=WidgetTree::new();
        tree.mount(outer.into()).unwrap();
        tree.layout(Constraints::tight(Size::new(200.,200.))).unwrap();
        let base=Instant::now();
        send(&mut tree,PointerPhase::Down,50.,50.,0,base);
        send(&mut tree,PointerPhase::Move,50.+dx,50.+dy,10,base);
        starts.get()
    };
    assert_eq!(run(0.,20.,0.),1,"unrotated horizontal positive control");
    assert_eq!(run(std::f32::consts::FRAC_PI_2,0.,17.),0,"below touch slop");
    assert_eq!(run(std::f32::consts::FRAC_PI_2,0.,20.),1,"rotated local-axis drag");
}
```

Append to a temporary copy of `crates/incular-gestures/tests/scale_slop.rs`,
as `audit_scale_velocity_tmp.rs`. Run
`cargo test --locked -p incular-gestures --test audit_scale_velocity_tmp audit_gesture_`.

```rust
#[test]
fn audit_gesture_synthesized_moves_do_not_feed_scale_velocity() {
    use std::time::Duration;
    let run=|synthetic:bool| {
        let ends=Rc::new(RefCell::new(Vec::new()));
        let result=ends.clone();
        let mut scale=ScaleGestureDetector::with_callbacks(None,Some(Rc::new(|_|{})),
            Some(Rc::new(move |d:ScaleEndDetails|result.borrow_mut().push(d.velocity.pixels_per_second.x))));
        let t=Instant::now();
        let event=|phase,x,ms,synthesized| RawPointerEvent {
            synthesized, ..RawPointerEvent::new(1,0,PointerDeviceKind::Touch,0,
              Offset::new(x,0.),phase,t+Duration::from_millis(ms))
        };
        scale.handle_raw(event(PointerPhase::Down,0.,0,false));
        for k in 1..=4 {
            scale.handle_raw(event(PointerPhase::Move,20.*k as f32,10*k,synthetic));
        }
        scale.handle_raw(event(PointerPhase::Up,80.,45,false));
        let x=ends.borrow()[0];
        x
    };
    assert!(run(false)>1000.,"ordinary velocity positive control");
    assert_eq!(run(true),0.);
}
```

## Synthesized scale velocity repair

C10 now matches the pinned authored reference: regular and synthesized
sequences finish with the same focal position, but only regular motion
feeds the release velocity. The reproducer was still failing at
2,000 px/s just before the repair. Ten focused Rust tests passed with
both Flutter probes. This checks raw touch recognizer inputs, not native
trackpad pan/zoom or resampling at the runtime boundary. C10 is an
authored behavior probe and does not increase the upstream case count.

## Mixed real and synthesized retained scale

A pinned mounted Flutter probe and an Incular Runtime test deliver
three real and two non-linear synthesized moves. Both listeners receive
the five provenance flags, both scale detectors report every geometric
position, and both end at (200,100) with a velocity of (2000,0) px/s
within 0.01. Runtime now forwards the raw samples with their supplied
timestamp, including the provenance bit, to retained scale. Twenty
focused Rust tests passed, including pressure, buttons and the input
allocation check. Native platform adapters are not assumed to
synthesize motion. This authored case is outside the upstream count.

## Synthesized horizontal drag and quick flicks

The two exact Flutter cases at `drag_test.dart:1519,1567` passed. The
mounted Incular horizontal recognizer uses the same positions and
timestamps: after a synthesized wobble the release still reports
(1000,0) px/s, and down plus two genuine moves still produces a
1000 px/s quick flick. Synthesized samples continue through motion
handling but are not fitting points. A warmed raw-input diagnostic
regression also verifies zero allocations over 200 moves. Fifteen
focused Rust tests passed. This does not cover native trackpad sampling.
