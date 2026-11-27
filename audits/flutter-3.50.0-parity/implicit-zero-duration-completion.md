# Completion of zero-duration implicit opacity

Authored probe on pinned Flutter 3.50.0
`ab2912035e160d9cad6306df307576da6128ebdb`. Both boxed and sliver
opacity variants jump to each new target and call the current onEnd
once. Construction at the initial target and a rebuild with no target
change do not complete a run.

Before this repair the retained sliver reached the requested alpha but
dropped the callback because it looked for completion only in a later
tick. A zero-duration controller completes synchronously. The sliver
now checks for a transition to completed during retargeting as well
as after later ticks, without reporting it twice. The box path already
uses the controller's status listener after the tween repair.

The native case reproduced the lost callback before this patch, then
passed on both paths. The two stale sliver assertions in the adjacent
suite now sample the actual shared opacity layer rather than requiring
an obsolete per-child opacity render node, including the 128/255
composited alpha at halfway. All 24 implicit widget tests, 14 completion
tests and 13 sliver opacity/layer tests passed. The two authored Flutter
variants earn no original-registration credit.
