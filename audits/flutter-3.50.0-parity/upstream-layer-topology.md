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
