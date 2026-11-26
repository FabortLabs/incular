# Animation repeats, traversal, restoration, flex, and image sampling

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

- Five exact AnimationController repeat cases match bounded and
  reversing values, status switch at 1000 ms, and finite counts
  (:873, :920, :1126, :1289, :1324). Count zero (:1267) diverges:
  Flutter debug rejects it; Incular keeps reversing through 425 ms
  and its future stays pending. Count one completes at 100 ms.
  Negative count and missing duration (:1278, :541) are reference-only,
  not representable via the Rust unsigned count and required duration.
  See `flutter350_upstream_animation_repeat` and
  `flutter350_upstream_repeat_invalid_count`.
- Focus traversal matches numerical order, lexical order, and skip
  under ExcludeFocusTraversal (:1279, :1331, :3691). Rust uses
  equivalent normalized Tab/Shift+Tab because the original Dart
  FocusNode methods have no public attached-node counterpart.
  A separate pinned, verified explicit-request comparison observes
  that Flutter does let the excluded child take focus directly.
  Incular leaves the previous host target in place and both nodes
  report primary focus. The direct-adoption path wrongly searches
  the traversal-filtered list. See `flutter_focus_traversal_upstream_parity`
  and `focus_exclude_traversal_reference_test.dart`.
- Restoration reads an existing bucket without dirtying it
  (:147); writing still persists. Claiming an empty new bucket
  (:172) does not request an early save in Incular, and flushData
  (:238) starts a write immediately even with a pending visual
  frame. Flutter schedules the empty claim and waits for that
  frame before flushing. Both paths retain the values after
  writes. These compare the injected storage request timing,
  not the engine channel/serialization. See
  `flutter350_restoration_dirty_flush_parity`.
- RenderFlex spaceBetween with spacing matches exact geometry
  in both axes (:494). No-baseline fallback to top, and a zero
  Baseline frame (:914, :115) match scoped geometry, not the
  direct baseline selector/native-view APIs. Negative spacing
  (:1256) asserts on Flutter's debug constructor and becomes
  zero gap in Incular's widget adapter. See
  `flutter_flex_baseline_geometry_parity`.
- The original ImageFiltered matrix and enabled/disabled cases
  report passes (:104, :158). A different authored pinned fixture
  proves actual matrix sampling: scaling a red-blue tiled child
  2× gives Flutter default blended pixels [191,0,64] and
  [64,0,191] at its seam; actual Windows wgpu keeps [255,0,0]
  and [0,0,255], like Flutter's explicit nearest control.
  Identity, disabling and scaled extent agree. Rust lowers the
  filter to a paint-space transform rather than a filtered
  intermediate raster. No original golden or native layer
  equivalence claim. See `flutter350_image_filter_matrix_sampling`
  and `image_filter_matrix_sampling_test.dart`.

### Matrix image filter sampling recheck

The default matrix ImageFiltered now selects the retained resampling
path at Flutter's Medium quality, mapped to Incular's linear sampler.
Windows wgpu now matches the two pinned 2× seam samples within one byte
per channel. Identity, disabling and the unpainted exterior still pass.
The retained test also checks the untransformed source, scaled output
bounds, linear sampling, and removal of the filter when disabled. This
does not establish mipmap behavior for minification or high-quality
sampling, and it does not upgrade the original golden coverage.

### Focus traversal from a directly focused skipped node

Hosted focus now adopts the explicit focus request in the earlier
fixture, and the excluded ancestor retains focus-within without
becoming primary. A follow-up found that the first Tab from that
middle node incorrectly jumped to the first candidate. A focused
node now participates as the ordering anchor, without making any
other excluded node a stop. Tab moves to the next sibling and
Shift+Tab to the preceding one, matching the pinned probe. A
node-level skipTraversal flag likewise skips normal traversal
while allowing direct focus and preserving the reverse anchor.
Both pinned probes and all 18 focused Incular traversal, hosted
focus and editor checks pass. The :3691 API comparison is still
partial: Incular drives the equivalent Tab intent, not Flutter's
attached FocusNode.nextFocus method.

### Nested and all-skipped focus groups

The pinned original group cases :3505, :3539, :3572, and :3622
all pass. Incular's retained checks verify that Tab does not move
an explicitly focused node when the group or all nodes are skipped,
that a nested unfocusable group never takes focus through either Tab
or a direct request, and that an empty RTL group leaves the only
focused sibling unchanged. All eight Rust tests in the traversal
target pass. These are partial ports: Tab does not expose the
original programmatic nextFocus return value, and the nested
test checks node focus rather than Focus.of and onFocusChange.

### Focus owns a node by default

A plain Focus previously collapsed to its child when no node was
supplied, so Tab skipped it altogether. Each mounted Focus now owns
a node unless one is supplied, and the node survives compatible
rebuilds. A pinned behavioral probe and the Rust counterpart verify
entering the middle Focus, preserving it across a child height change,
and then continuing to the following control. Initial autofocus and
supplied-node behavior still pass. All 29 focused Rust checks passed;
this authored verification does not add an original registration.

### Explicit restoration flush follows scheduled frames

The :238 timing mismatch is repaired. Application flushes wait for
a scheduled framework frame or a native redraw, include changes made
by post-frame callbacks, and bypass the debounce when no frame is
scheduled. Unscheduled headless layout does not stall a save, and
suspend and shutdown can still force persistence without a frame.
All 26 selected Rust tests passed. The separate empty-bucket dirty
case (:172) is not part of this fix; the engine channel and snapshot
format are not claimed equivalent.

### Empty restoration buckets survive a save

The :172 behavioral mismatch is repaired: claiming a new empty scope
requests a save before its first value write. The optional, backward-
compatible snapshot field records bucket paths separately from values.
Reopening an empty persisted bucket is clean; releasing a live lease
does not delete data. Removing an empty subtree updates persistence
without deleting similarly prefixed siblings. The pinned originals
:147, :172 and :238 pass, and the five Rust timing/bucket tests plus
23 restoration regressions pass. The separate same-turn handoff
port still reports the existing strict duplicate-lease mismatch.
The Flutter engine binary format is outside these behavioral matches.

### Multi-window restoration redraw isolation

A separately reproduced Incular integration bug let an unrelated
primary-window frame satisfy a deferred save before the scheduled
secondary window rendered. The save now waits for the redraw owners
captured at the request, includes the secondary frame's updated value
in the first snapshot, and does not wait for later unrelated redraws.
Closing, hiding, or making the awaited window zero-size releases its
wait; idle and headless retry still pass. This is an Incular host
check, not an extra Flutter registration.

## Current repeat count validation

Pinned :1267 and the native regression now reject zero in debug builds before
changing the controller or canceling an existing ticker. The extended native
test checks both the next-frame and timestamped entry points and shows an
already-running controller progressing from .25 to .5 after rejection.
`None`, not zero, is the unbounded repeat request. The five native finite
and indefinite repeat controls also passed.


## Restoration disposition recheck

On Nov 26 both original pinned cases, `restoration_bucket_test.dart:172`
and `restoration_test.dart:238`, passed again, along with the seven
`flutter350_restoration_dirty_flush_parity` cases. :238 is a full
behavioral pair: the boundary is the start of observable persistence,
with zero saves before a queued frame, exactly one at its end, and an
immediate save while idle. The format and engine IPC are not compared.
:172 matches the original empty-bucket dirty and value-save assertions;
it stays partial because Incular does not expose Flutter's debug-owner
metadata. The separate duplicate live-lease handoff still fails its
retained test and is not included in either disposition.
