# Paired slice: controller listener and status lifecycle

Flutter 3.50.0 `ab2912035e` vs Incular `c647cb85`.

| ID | Case | Flutter | Incular |
| --- | --- | --- | --- |
| C34 | Value listener A removes later listener B during dispatch | [A] | [A,B] |
| C35 | Controller at .5 animateTo(.5) | Completed status, still .5, zero value callbacks | Completed status, still .5, one value callback |
| Control | Use a per-run Decelerate override, stop, reset to 0, then repeat at 250 ms of a 1 s cycle | .25 | .25 |

Flutter checks listener membership after snapshotting
(`animation/listener_helpers.dart:144–161,237–243`); Incular clones callbacks
and invokes the cloned batch with no membership recheck
(`crates/incular-animation/src/controller.rs:1195–1209`). This is separate
from the positive DependencySource listener-removal control, which uses
a per-observer active bit. For the equal target, both report terminal status
but only Incular fires unchanged value (`controller.rs:1018–1056` vs
Flutter `animation/animation_controller.dart:668–681`).

The repeat control clarifies manual finding A-01: Flutter per-run curve
does not persist, and neither does Incular's explicit `animate_to_curved`
override. The Rust `set_curve` controller-default extension flows into
`repeat_at` (`controller.rs:930,1035–1065,1070–1098`), whereas Flutter
has no equivalent persistent default setting. Do not label the ordinary
per-run sequence a mismatch. Other reverse/count terminal notifications and
ticker disposal remain open.

## Notification repair

C34 and C35 now pass in the focused controller regression target. Value
and status registrations removed during notification are skipped even
if they were in the current snapshot; newly registered callbacks wait
for the next notification. An equal-target run only reports a change in
terminal status. When a zero-duration run has a different target, the
value callback sees the preceding status, followed by the terminal
status callback. The pinned forward and reverse cases agree; a status-only
change still invalidates retained reactive readers once.

Four targeted Rust lifecycle checks and all ten public-API checks passed.
Five authored Flutter controls, including the mutation and direction-order
extensions, also passed.

## Reproduction

Pin the scratch Flutter package and run
`flutter test --no-pub test/controller_slice_test.dart`; all three passed.
Rust used `cargo test --locked -p incular-animation --test audit_controller_tmp -- --nocapture`;
the control passed and both parity assertions failed. Rust source target
was removed.

```dart
import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';
void main(){
  testWidgets('removed controller listener does not run in same notification',(t)async{
    final c=AnimationController(vsync:t,duration:const Duration(seconds:1));
    try {
      final calls=<String>[];
      void b()=>calls.add('B');
      c.addListener((){calls.add('A');c.removeListener(b);});
      c.addListener(b);
      c.value=0.5;
      expect(calls,['A']);
    } finally {c.dispose();}
  });
  testWidgets('same-target animation notifies terminal status, not an unchanged value',(t)async{
    final c=AnimationController(vsync:t,duration:const Duration(seconds:1),value:.5);
    try {
      final statuses=<AnimationStatus>[];
      var values=0;
      c.addListener(()=>values++);
      c.addStatusListener(statuses.add);
      c.animateTo(.5);
      expect(c.value,.5);
      expect(statuses,[AnimationStatus.completed]);
      expect(values,0);
    } finally {c.dispose();}
  });
  testWidgets('previous per-run curve does not affect repeat',(t)async{
    final c=AnimationController(vsync:t,duration:const Duration(seconds:1));
    try{
      c.animateTo(1,curve:Curves.decelerate);
      c.stop();c.value=0;c.repeat();
      await t.pump();
      await t.pump(const Duration(milliseconds:250));
      expect(c.value,closeTo(.25,.0001));
    }finally{c.dispose();}
  });
}
```

```rust
use incular_animation::{AnimationController,AnimationStatus,Curve};
use std::{cell::{Cell,RefCell},rc::Rc,time::{Duration,Instant}};
#[test]
fn removed_listener_skips_current_dispatch(){
    let c=AnimationController::new(Duration::from_secs(1));
    let calls=Rc::new(RefCell::new(Vec::new()));
    let b=Rc::new(Cell::new(None));
    let captured=b.clone();
    let controller=c.clone();
    let a_calls=calls.clone();
    let a=c.add_listener(move |_|{
        a_calls.borrow_mut().push("A");
        if let Some(id)=captured.take(){controller.remove_listener(id);}
    });
    let b_calls=calls.clone();
    b.set(Some(c.add_listener(move |_|b_calls.borrow_mut().push("B"))));
    c.set_value(0.5);
    c.remove_listener(a);
    assert_eq!(*calls.borrow(),["A"]);
}
#[test]
fn same_target_notifies_status_only(){
    let c=AnimationController::new(Duration::from_secs(1));
    c.set_value(0.5);
    let status=Rc::new(RefCell::new(Vec::new()));
    let s=status.clone();
    c.add_status_listener(move|v|s.borrow_mut().push(v));
    let count=Rc::new(Cell::new(0));
    let seen=count.clone();
    c.add_listener(move |_|seen.set(seen.get()+1));
    c.animate_to(Instant::now(),0.5);
    assert_eq!(c.value(),0.5);
    assert_eq!(*status.borrow(),[AnimationStatus::Completed]);
    assert_eq!(count.get(),0);
}
#[test]
fn a_previous_per_run_curve_does_not_change_repeat(){
    let start=Instant::now();
    let c=AnimationController::new(Duration::from_secs(1));
    c.animate_to_curved(start,1.,None,Curve::Decelerate);
    c.stop();c.set_value(0.);c.repeat(start,false);
    c.tick(start+Duration::from_millis(250));
    assert!((c.value()-0.25).abs()<0.0001);
}
```
