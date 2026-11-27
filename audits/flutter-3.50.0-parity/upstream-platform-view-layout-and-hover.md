# Native view placement and hover at the host boundary

Five pinned rendering originals pair against the retained Runtime's
actual placement and pointer commands. The view fills the largest
allowed 100 × 100 size after a constraints change, even when the
minimum is only 50 × 50. Before layout, neither an initial mount
nor a controller replacement sends placement. The first laid-out
frame reports the origin and the 800 × 600 size exactly once to the
active controller, after creation.

The hover pair starts with device add, moves at (10,10) with no
buttons pressed, and removes the device. The active controller
receives the move at that local position. Both pinned hover
registrations use the helper's default mouse device, including
the one titled “touch hover”; these matches do not claim physical
touch hover. The existing native lifecycle control also passes.

These checks use the host command boundary, not a real Android,
UIKit, or AppKit view. Accessibility-tree grafting, native
compositing, and gesture-arena acceptance/rejection remain separate.
