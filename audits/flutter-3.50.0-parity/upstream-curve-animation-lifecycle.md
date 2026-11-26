# Animation wrapper clocks and lifecycle

Seven pinned `animation/animations_test.dart` registrations pair:

- :77 a reverse wrapper's listener fires on value change, not on
  registration or after removal; its ordinary debug description is
  a single line.
- :161, :191 and :221 real Mean, Max and Min graph nodes report the
  original numeric values, forward one changed value, unsubscribe,
  and can still be read after a later change to the other parent.
- :277 and :367 a curved wrapper and its reverse follow every 10 ms
  sample of a 100 ms forward and 50 ms reverse controller. Both agree
  on endpoint values, the one-frame time origin on reverse, and
  complementary values/statuses. The controller's completion
  condition is strictly after the endpoint; value is already at the
  endpoint while status is still moving on the exact boundary.
- :326 an interval curve selects forward at 0.5 before completion
  and reverse at 0.5 after it. Disposing unsubscribes the private
  direction tracker, so after parent dismissal and a new forward
  run at 0.5 the wrapper still uses its latched reverse curve.
  It continues to read the live parent and forward value events.

The three focused native targets passed 28 cases. Native diagnostic
assertions cover one-line descriptions rather than Dart class spelling.

## Wrapper diagnostics and object lifetimes

Five more pinned originals pair. Constant, forward/reverse curved,
and detached/attached proxy wrappers have useful single-line debug
descriptions (:27, :51). A train starts with a pending next train,
switches without waiting for a read, and describes the new active
train with no pending next train (:97). Explicit disposal stops both
clock subscriptions; further crossings do not notify it.

CurvedAnimation and TrainHoppingAnimation emit one debug create and
one debug dispose event with the same identity (:500, :122). Repeated
disposal followed by Rust drop does not emit another event. Release
builds do not store these debug lifetimes. Five original Flutter
and 31 focused native tests passed.
