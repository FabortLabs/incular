# TapRegion and accessibility actions (backfill W09)

All three selected originals in widgets/tap_region_test.dart passed on pinned Flutter 3.50.0. The Rust target sends actual Runtime semantic actions to generated semantic nodes by label; it does not simulate taps with a pointer event. One Rust test passes and three intentionally fail.

- :1262 and :1358: an outside passive text node receives semantic tap or long press; Flutter notifies the region once and Incular does not notify it.
- :1313: targeting the inside passive node produces no outside notification in either framework.
- Authored callback control: a painted, hit-testable outside node advertises the action and has a real handler. Flutter executes that handler and the region's outside callback once per tap/long press. Incular executes the target handler once but still never notifies the region. This rules out passive nodes lacking handlers as the entire explanation. A non-hittable empty sized box is not an equivalent control because Flutter classifies region membership using a real hit test.

This checks framework semantics dispatch, not the OS screen-reader bridge. Run the authored Flutter control with run-reference-probe.ps1 -Test tap_region_actionable_semantics_test.dart, and cargo test --locked -p incular-runtime --test flutter350_tap_region_semantic_actions -- --nocapture.

## Remediation

Semantic tap and long press now notify each TapRegionSurface by hit-testing
its own subtree at the target semantic node's world-space center. The
surface delivers outside callbacks before inside callbacks. This is a
down-only notification, not a pointer stream, so it neither reaches
raw pointer listeners nor starts a recognizer. A consuming region cannot
prevent the target accessibility action from running. The same route-current
filter and grouping rules apply as for real pointer events.

The added paired actionable control checks callback order, the reported
center, exactly one target invocation, and the absence of an up or raw
pointer callback. Incular initially called only the target. The original
passive inside, outside, and long-press controls are preserved. These
comparisons cover framework dispatch only; they do not certify host
screen-reader adapters.

The Linux Dot run passed all five focused semantic tests and eight
pointer/group/route controls. The two authored probes and three exact
original Flutter registrations (:1262, :1313, :1358) passed at the
unchanged pin. Historical dispositions remain unchanged.
