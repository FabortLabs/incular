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

### Matrix image filter minification

The authored pinned `image_filter_matrix_minification_test.dart`
reads actual Flutter RGBA. At a 4:1 horizontal reduction, nearest and
Low leave a three-red/one-blue stripe solid red. Medium (also the
default) returns `[191, 0, 63, 255]` instead of aliasing away the blue.

Incular preserves the Medium matrix policy instead of collapsing it to
Low. On wgpu, a minifying transform builds only the needed mip levels
from the isolated source and linearly samples between levels. Odd-sized
source texels contribute by area; nearest and Low do not allocate mip
levels, and neither does magnification. The retained final filter cache
keeps the result when source, matrix, and geometry are unchanged.
The optional D3D11 renderer's single-level fallback is unchanged.

The focused GL/llvmpipe test builds and reads the production shader's
mip chain, verifies the Flutter sample to one byte per channel, tests
fractional levels, and checks pixels outside the filtered shape remain
transparent. The widget tests also check the retained default mode.
This is not full original golden equality or a different GPU backend
comparison.

### High-quality matrix transforms

Pinned Flutter's High setting uses the Mitchell-Netravali kernel
with B=C=1/3. The authored probe reads red/blue edge pixels: the two
inner samples change from `[191,0,64]` and `[64,0,191]` at Low to
`[193,0,62]` and `[62,0,193]` at High. Opaque interiors and pixels
outside the filtered bounds retain their original values.

The production wgpu resample shader now applies that separable cubic
kernel for High. Its output remains premultiplied and bounded; negative
lobes cannot produce negative alpha or paint beyond finite source
coverage. The GL readback and retained Transform tests verify that High
does not lower to Low. Ordinary image draws keep their existing path.
This comparison adds no original golden registration.

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
metadata. At the time of this check the separate live-lease handoff
failed; [the later ownership audit](upstream-restoration-scope-ownership.md)
records the explicit staged replacement that now passes.

### Programmatic focus group parity

Eight pinned originals now have matched native fixtures:
`focus_traversal_test.dart:3364, :3374, :3505, :3539, :3572,
:3622, :3691, :3733`. Mounted traversal groups and
ExcludeFocusTraversal introduce no accessibility node on an empty
scene. A plain or nested unfocusable group denies explicit requests
from both focus nodes found through their mounted contexts, without
calling onFocusChange. Its eligible sibling keeps primary focus.

The runtime now exposes programmatic next/previous traversal and
reports whether primary focus moved. All-excluded groups and an
empty RTL group do not move focus; an exclusion between eligible
targets is bypassed. The native API uses the window's current
primary focus (the initiating node in these originals), not every
possible FocusNode-relative traversal policy. Forty-four related
native tests passed, including keyboard input and editor selection.

## Owned restoration manager memory events

The pinned `services/restoration_test.dart:16` registration passed.
The native fixture starts a real application with an in-memory
restoration store. The runtime-owned manager reports one creation;
cloning the restoration capability neither creates another manager
nor drops the first early when the app is released. The final handle
releases that same allocation and produces one disposal. All 15
focused persistence, bucket ownership, and dirty-flush checks passed.

### Explicit matrix quality for backdrops

A pinned Flutter widget probe puts 32×16 red/blue stripes behind a
clipped BackdropFilter at DPR 1. Quarter-scale Low returns solid red;
explicit Medium and the unchanged default return `[191, 0, 63, 255]`.

`ImageFilter::matrix_with_quality` carries all four policies through
subtree and backdrop filters. A direct or composed backdrop uses the
same production matrix sampler as an isolated subtree. Only Medium
minification adds mip work, and its source copy, mip passes, and final
sample share the effect encoder. An explicit quality change keeps the
layer ID, but invalidates that layer's filtered-result signature.

The wgpu offscreen test uses the backdrop affine mapping and actual
readback of the striped capture, matching the pinned bytes to one
channel level. The retained widget tests compare all four modes and
the generation change inside a composed backdrop, with the color
stage still in order. This compares an isolated offscreen capture,
not the full original golden or the optional D3D11 fallback.
