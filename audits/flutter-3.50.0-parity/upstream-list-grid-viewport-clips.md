# List and grid viewport clipping

Nine pinned clipBehavior originals passed: ListView, ListView.builder,
ListView.custom and ListView.separated in `list_view_test.dart`, and
GridView, GridView.builder, GridView.custom, GridView.count and
GridView.extent in `grid_view_test.dart`.

The native fixtures inspect the retained viewport and its submitted
display list. Both default views initially use a hard-edge rect when
the content overflows. Updating the same viewport to anti-aliased
coverage changes that actual edge; disabling clipping removes the
command. Constructor-specific probes distinguish configured clipping
from actual overflow: a single fixed-count tile fits and emits no
viewport clip, even though the viewport retains the anti-aliased
setting. An oversized tile and multi-row overflow both emit the
configured clip. All eleven focused native tests passed. No GPU
pixel-equivalence claim is attached to this display-list comparison.

This adds seven new paired registrations; the default and builder
GridView registrations were previously paired and were rechecked.
