# Zero-area scroll viewports

The pinned originals `custom_scroll_view_test.dart:169`,
`list_view_test.dart:1178`, `grid_view_test.dart:483`, and
`shrink_wrapping_viewport_test.dart:60` passed. The native fixtures
mount the same centered zero-sized custom/list/grid viewports and set
zero physical metrics for a shrink-wrapping viewport seeded with the
original 5-pixel offset. This checks layout, not the policy of
Flutter's immutable fixed-offset object.

The authored Flutter grid probe distinguishes being cached from being
visible: tile zero exists offstage, while tile one is not constructed
at zero stride. Incular had constructed the entire first row and
submitted zero-window glyphs. It now builds only the leading tile
and disconnects the children of a sliver viewport with zero main-axis
paint extent. Expanding the viewport reconnects and paints the retained
child with the same identity. Previously kept-alive siblings are not
discarded; a second native case verifies that only explicitly retained
siblings survive the zero-stride transition and none paints there.

All ten focused zero-area, grid keep-alive, cache, and scrolling
cases passed. There is no platform compositor or golden claim.
