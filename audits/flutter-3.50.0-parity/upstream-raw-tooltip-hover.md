# RawTooltip hover handoff

Both exact registrations passed in pinned Flutter 3.50.0
`packages/flutter/test/widgets/raw_tooltip_test.dart`:

- Line 889: a cold hover first waits all 700 ms. Moving straight to
  the next target on that display frame shows its bubble immediately
  and removes the previous bubble.
- Line 952: one or more rapid reentries before the scheduled hide
  keep the tooltip visible. A later uninterrupted exit settles it.

Native runtime tests replay pointer events against the mounted
overlay and confirm measured element presence, not just controller
flags. Further controls preserve the delay after a stale registry
entry, preserve independent mouse-device ownership of another
tooltip, resume an in-flight reverse on pointer reentry, and check
the immediate Rust builder's same-packet exit. These additional
controls are not extra upstream credit. 39 focused native checks
passed, including the animation, trigger, placement and Escape suites.
