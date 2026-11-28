# Retained layer traversal, clip bounds, and reparenting

Two pinned Flutter 3.50.0 originals in `rendering/layers_test.dart` passed.
The native fixtures reproduce the nine-node paint-order traversal before
and after removing an interior branch, and describe the local clip bounds
of rect, rounded rect, rounded superellipse and path layers. A non-clip
has no bounds. A separate curve check distinguishes control-point bounds
from the tighter bounds used for culling.

An authored pinned raster probe moves one retained picture across sibling
offset layers. Alpha at the two sites changes from [255,0] to [0,255].
The native baseline incorrectly flattened the picture in both places
after adoption. The retained arena now unlists an adopted child from its
former parent, updates each former parent once, and invalidates its old
bounds. The native test verifies exactly one draw at the new location
while retaining the same picture identity.

All 39 focused native layer, compositor, sliver, keep-alive and proxy
cases passed. The authored reparenting comparison adds no original
registration credit. Only Flutter's raster was run; Incular's native
test validates the render stream and placements.

## Leader links and retained transforms

Five pinned `rendering/layers_test.dart` cases passed. Replacing an
attached leader's link retires its old publication; removal clears
the new one as well. Two attached owners can detach second-before-first
with no stale publication. After repainting an ancestor's independent
picture, the retained leader's own transform and generation persist
and its picture still lands at the intended offset. The native check
also exercises a nonzero local translation.

Connecting and disconnecting followers changes their actual picture
placements without replacing the leader's picture or advancing its
published generation. These two registrations remain partial: there
is no equivalent engine dirty bit, and number of link resolution
passes is separate from picture reuse. All 16 focused compositor
topology and mutation tests passed.

## Picture ownership and hidden boundaries

Five additional pinned layer registrations passed. Clearing a
PictureLayer's payload drops the previously owned display list.
Removing the layer releases its unshared payload and the stale
generational ID cannot readopt a new resource. Native tests verify
actual weak references and flattened draws, not a guessed object
count. An independently held display list survives removal only
until its last owner drops.

A zero-opacity nested boundary contributes no drawing and returns
with the identical stored picture when restored to 0.5, without a
repaint. That is partial because Flutter visibly detaches its
boundary, whereas Incular leaves its retained subtree structurally
owned. Flutter's handle-count and manual-dispose assertions likewise
remain partial: Rust separates detachment from removal, and uses Arc
lifetime instead of allowing manual disposal of a live reference.
The two picture-release cases pair. This checks CPU picture lifetime,
not GPU fence retirement. All six focused tests passed.

## Leaders hidden by a zero-alpha ancestor

The authored pinned Flutter probe uses one target and a sibling
follower with `showWhenUnlinked: false`. At opacity 1.0, 0.0,
0.001, and 0.5 the link and actual tap delivery alternate visible,
hidden, hidden, visible. Incular used to publish the retained leader
even when its ancestor skipped painting, leaving the follower able
to draw and receive hits at a hidden anchor.

Link publication now uses the same effective-alpha threshold as
flattening. The runtime test verifies retained follower identity,
draw commands, real hit targets, and restored 20×20 placement.
A direct compositor control also verifies that a hidden first
candidate cannot displace a visible target. All 18 focused tests
passed. This authored probe does not add an upstream registration.

## Relinking during layout and repainting a sibling

Both pinned `widgets/composited_transform_test.dart` cases passed.
Rebinding the target and follower inside the layout callback kept
the same keyed 20×20 child at (118,451), and native paint confirmed
that placement. The old link was cleared and the new one published.
A separate fixture changed a clipped leader's left padding from 10
to 0, then repainted only its sibling. The leader's transform and
size stayed correct through all three frames, and the sibling
produced its expected fill exactly once. All ten focused native
composited transform tests passed.

## Retained leaders detached from the scene

The authored pinned Flutter layer probe keeps an ancestor and its
leader alive with handles. Removing and reattaching the ancestor
clears and restores the link; detaching the root also clears it.

Before the Incular fix, removing a retained branch from the root
left one ghost follower picture in the published display list.
A departing branch now clears only links published by its own
leaders. Native tests check actual follower paint and placement,
retained identity, root detachment and replacement, and ancestor
removal. A shared-link control verifies that a departing non-owner
cannot clear another tree's publication and that moving the root
into an already attached branch leaves that branch linked.

All 22 focused topology and composition callback tests passed.
A wider rendering target returned 70 passes and two curve-bounds
failures; rerunning against the pre-fix compositor reproduced
both unchanged. The authored comparison adds no upstream credit.

## Scene ownership of follower transforms

The two authored Flutter cases confirm that a follower cannot import
the transform of a leader owned by a different scene, regardless of
its unlinked visibility flag. Attaching the follower under the leader's
scene instead restores its exact projection. Flutter debug rejects the
invalid configuration; Incular safely uses the local unlinked policy
rather than surfacing the Flutter assertion.

Before the fix a mounted second Incular runtime accepted hits at a
foreign anchor, even with `show_when_unlinked: false`. Paint, pointer
placement, and semantic bounds now consume the same scene-local
publication. Unlinked visible content stays at (80,70); hidden content
has no picture, hits, or semantic label. A newly mounted local leader
positions the follower at (61,27), and moving the foreign leader on a
later frame cannot move that local target. Two scene publications are
independent and expire with their owning trees. Same-scene tooltip and
overlay controls still pass; these routes split the display list after
link resolution rather than importing a second tree's coordinates.

Both pinned probes and 105 focused native compositor, follower, overlay
and tooltip tests passed. This comparison adds no upstream registration.
