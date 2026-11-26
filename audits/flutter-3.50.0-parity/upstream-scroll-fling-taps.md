# Flutter 3.50 repeated flings and stop taps

The pinned `widgets/scroll_interaction_test.dart:9` original passed.
The paired runtime test sends two 200-pixel touch flings at 500 px/s
through the same mounted position. Each fling advances after release,
becomes idle on the five-second frame, and the second advances past
the first's settled offset.

The originals `widgets/scrollable_fling_test.dart:101` and `:136`
passed. Their Rust fixtures reproduce the stock tester's 50-sample
touch fling and the visible 14-pixel rows. The first ordinary tap
selects row 21. Before the repair, a stop tap incorrectly activated
row 35 while stopping the motion. Now it only stops the fling at the
current offset. The next contact selects row 35. If the fling is
left to settle instead, the first new tap selects row 49.

Flutter marks touch ballistics and driven animateTo motion as
ignoring child pointer hits. Incular now applies that policy during
the shared viewport hit traversal, so raw listeners and retained taps
are affected together without removing the viewport's own recognizer.
The live metric attachment and activity token guard against stale
drivers. Idle programmatic settles do not suppress child hits; the
trackpad exception does not inherit touch suppression.

An authored Flutter probe and matching runtime control also verified
that a stop tap during animateTo reaches neither the child's
onPointerDown nor its onTap, and a later tap reaches both. All seven
focused activity tests and eleven runtime pointer tests passed.
