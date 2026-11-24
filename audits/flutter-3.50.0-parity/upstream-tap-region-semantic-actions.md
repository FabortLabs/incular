# TapRegion and accessibility actions (backfill W09)

All three selected originals in widgets/tap_region_test.dart passed on pinned Flutter 3.50.0. The Rust target sends actual Runtime semantic actions to generated semantic nodes by label; it does not simulate taps with a pointer event. One Rust test passes and three intentionally fail.

- :1262 and :1358: an outside passive text node receives semantic tap or long press; Flutter notifies the region once and Incular does not notify it.
- :1313: targeting the inside passive node produces no outside notification in either framework.
- Authored callback control: a painted, hit-testable outside node advertises the action and has a real handler. Flutter executes that handler and the region's outside callback once per tap/long press. Incular executes the target handler once but still never notifies the region. This rules out passive nodes lacking handlers as the entire explanation. A non-hittable empty sized box is not an equivalent control because Flutter classifies region membership using a real hit test.

This checks framework semantics dispatch, not the OS screen-reader bridge. Run the authored Flutter control with run-reference-probe.ps1 -Test tap_region_actionable_semantics_test.dart, and cargo test --locked -p incular-runtime --test flutter350_tap_region_semantic_actions -- --nocapture.
