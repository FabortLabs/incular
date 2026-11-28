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
