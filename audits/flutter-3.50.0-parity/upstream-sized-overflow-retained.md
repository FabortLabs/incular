# SizedOverflowBox passes the parent's constraints to the child

Pinned `widgets/overflow_box_test.dart` registrations :115, :142
and :169 passed. All three are now compared through their full
observable assertions in Incular.

- Absolute top-right positioning in RTL leaves the 50×50 child at
  (400,250), with its parent at (350,250) and sized 100×100.
- Inherited bottom-start is direction-sensitive. In RTL it places
  the child at (400,300); flipping the retained environment to LTR
  moves it to (350,300), and switching back restores (400,300).
  The Rust and Flutter probes keep the same render object.
- A placeholder mounted in a host view with zero physical area
  lays out to a zero area without a crash. The Incular test sends
  actual platform metrics into Runtime and completes a frame with
  that view size, then restores the original view without remounting.
  A native OS window is not required for either test harness.

The additional pinned constraints probe keeps a 200×150 child at
its requested size under loose host constraints, despite the
100×100 wrapper. Its centered bounds are (300,225,200,150).
Previously the composition lowered to a tightly sized intermediate
box and capped the child at 100×100. The new retained wrapper
accepts the original constraints, positions the child against its
own constrained requested size, and forwards a real baseline with
the same placement offset. It does not depend on OverflowBox's fit.
