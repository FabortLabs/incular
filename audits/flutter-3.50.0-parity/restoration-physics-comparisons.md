# Paired slices: persistence removal and simulation precision

Flutter 3.50.0 at `ab2912035e`; Incular framework `c647cb85`, audit
HEAD before these probes `03467cbd`. Both sides ran focused tests.

| ID | Case | Flutter actual | Incular actual |
| --- | --- | --- | --- |
| C18 | Unregister/remove a draft, then edit still-live app state | Store remains absent | Removed key is written again as "after" |
| C19 | 500 ms spring with bounce 0 | Critically damped | Overdamped |
| C20 | Bounded friction at t=1.701 s, drag .5, start 8191, v=1, bounds 8190..8192 | x=8191.998961235408, not done | x=8191.9990234375, done |

C18 controls agree on write before removal, immediate absence after removal,
and subsequent mutation of live in-memory state. Flutter's comparator retains
the underlying editable controller, not the unregistered RestorableProperty
wrapper (that wrapper asserts if used after unregister). Incular's generic
Restorable<T>::remove documents that it stops restoring but only deletes the
current entry: set still invokes the retained writer. The separate text
controller's unbind operation stops writes without deleting, so an app can
combine it with explicit removal for that specific type. References: Flutter
`widgets/restoration.dart:772–812,831–834`,
`widgets/restoration_properties.dart:402–417,495–502`; Incular
`crates/incular-runtime/src/restoration.rs:1186–1209,1217–1234`,
`crates/incular-text/src/editing.rs:420–423`. This is not a claim that
Flutter and Incular use the same storage representation.

C19 controls agree at bounce -0.1 (overdamped) and 0.1 (underdamped).
Flutter's double precision reaches a zero discriminant for the 500 ms
case; Incular stores stiffness and damping as f32 before computing its
discriminant. C20 controls agree at 1.700 s (not done) and 1.702 s (done).
The intermediate returned positions straddle the 0.001 distance tolerance.
References: Flutter `physics/spring_simulation.dart:70–81,284–294`,
`physics/friction_simulation.dart:186–195`; Incular
`crates/incular-animation/src/physics.rs:137–153,179–187,627–644`.
Other durations, precision policy and simulation sweeps remain open.

## Temporary paired fixtures

Use the pinned scratch Flutter package from [paired comparisons](focused-comparisons.md).
Run each of `flutter test --no-pub test/restoration_slice_test.dart` and
`flutter test --no-pub test/physics_slice_test.dart`. All three Flutter
assertions groups passed. The corresponding Rust assertions failed after their
controls, as recorded above. The scratch logs are under
`.tmp/flutter350-full-audit/`; temporary Rust targets were removed.

`test/restoration_slice_test.dart`:

```dart
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Probe extends StatefulWidget {
  const Probe({super.key});
  @override State<Probe> createState() => ProbeState();
}
class ProbeState extends State<Probe> with RestorationMixin {
  final draft = RestorableTextEditingController(text: 'default');
  @override String get restorationId => 'probe';
  @override void restoreState(RestorationBucket? oldBucket, bool initialRestore) {
    registerForRestoration(draft, 'draft');
  }
  void detach() => unregisterFromRestoration(draft);
  bool get hasDraft => bucket!.contains('draft');
  String? get savedDraft => bucket!.read<String>('draft');
  @override Widget build(BuildContext context) => const SizedBox.shrink();
  @override void dispose() { draft.dispose(); super.dispose(); }
}
void main() {
  testWidgets('unregistered mutable draft stays absent', (tester) async {
    final root = RestorationBucket.empty(restorationId: 'root', debugOwner: null);
    final key = GlobalKey<ProbeState>();
    try {
      await tester.pumpWidget(UnmanagedRestorationScope(bucket: root, child: Probe(key: key)));
      final state = key.currentState!;
      final editor = state.draft.value;
      editor.text = 'before';
      expect(state.hasDraft, isTrue);
      expect(state.savedDraft, 'before');
      state.detach();
      expect(state.hasDraft, isFalse);
      editor.text = 'after';
      expect(editor.text, 'after');
      expect(state.hasDraft, isFalse);
    } finally {
      await tester.pumpWidget(const SizedBox.shrink());
      root.dispose();
    }
  });
}
```

Append to a temporary copy of `crates/incular-runtime/tests/restoration.rs`
called `audit_restoration_slice_tmp.rs`; run
`cargo test --locked -p incular-runtime --test audit_restoration_slice_tmp audit_removed_restorable_stays_absent_after_an_edit -- --exact`.

```rust
#[test]
fn audit_removed_restorable_stays_absent_after_an_edit() {
    let scope = RestorationScope::root(Rc::new(MemoryBackend::default()));
    let id = key("draft");
    let draft = Restorable::from_scope(scope.clone(), id.clone(), String::from("default"));
    assert!(draft.set("before".into()));
    assert_eq!(scope.get_json(&id), Some(json!("before")));
    draft.remove();
    assert_eq!(scope.get_json(&id), None);
    assert!(draft.set("after".into()));
    assert_eq!(draft.get(), "after");
    assert_eq!(scope.get_json(&id), None);
}
```

`test/physics_slice_test.dart`:

```dart
import 'package:flutter/physics.dart';
import 'package:flutter_test/flutter_test.dart';
void main(){
  test('zero bounce gives critical damping',(){
    for (final entry in [(-0.1,SpringType.overDamped),(0.1,SpringType.underDamped),(0.0,SpringType.criticallyDamped)]) {
      final spring=SpringDescription.withDurationAndBounce(duration:const Duration(milliseconds:500),bounce:entry.$1);
      final sim=SpringSimulation(spring,0,1,0);
      print('bounce=${entry.$1} type=${sim.type}');
      expect(sim.type,entry.$2);
    }
  });
  test('bounded friction respects tolerance around 8192',(){
    final sim=BoundedFrictionSimulation(.5,8191,1,8190,8192);
    for (final entry in [(1.700,false),(1.702,true),(1.701,false)]) {
      print('time=${entry.$1} x=${sim.x(entry.$1)} done=${sim.isDone(entry.$1)}');
      expect(sim.isDone(entry.$1),entry.$2);
    }
  });
}
```

Temporary `crates/incular-animation/tests/audit_physics_slice_tmp.rs`:
`cargo test --locked -p incular-animation --test audit_physics_slice_tmp -- --nocapture`.

```rust
use incular_animation::{BoundedFrictionSimulation, Simulation, SpringDescription, SpringType};
use std::time::Duration;
#[test]
fn zero_bounce_gives_critical_damping() {
    for (bounce,expected) in [(-0.1,SpringType::OverDamped),(0.1,SpringType::UnderDamped),(0.,SpringType::CriticallyDamped)] {
        let s=SpringDescription::with_duration_and_bounce(Duration::from_millis(500),bounce);
        println!("bounce={bounce} type={:?}",s.spring_type());
        assert_eq!(s.spring_type(),expected);
    }
}
#[test]
fn bounded_friction_respects_tolerance_near_8192() {
    let sim=BoundedFrictionSimulation::new(0.5,8191.,1.,8190.,8192.);
    for (ms,expected) in [(1700,false),(1702,true),(1701,false)] {
        let time=Duration::from_millis(ms);
        println!("ms={ms} x={:?} done={}",f64::from(sim.position(time)),sim.is_done(time));
        assert_eq!(sim.is_done(time),expected,"ms={ms}");
    }
}
```
