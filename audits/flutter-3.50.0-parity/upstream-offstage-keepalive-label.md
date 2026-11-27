# Updating an offstage kept-alive row without exposing its semantics

Flutter 3.50.0, `ab2912035e160d9cad6306df307576da6128ebdb`:
`packages/flutter/test/widgets/semantics_keep_alive_offstage_test.dart:12`.

The pinned test starts near the end of an eager list, parks the stateful final
row by scrolling to the top, adds previously unlaid-out padding while parked,
updates its text from Foo to Bar, and scrolls back. Only the visible row may
publish a label. The Flutter test passed. Incular's mounted runtime test
`flutter350_offstage_keepalive_semantics` exercises the same sequence,
checks that the row keeps its element identity and the newly mounted padding
has not been laid out offscreen, and verifies the semantics tree before and
after the return. It passed with the production fix.

The mixed-height eager list previously started position reconciliation using
an estimate even for explicitly sized rows wrapped by a colored box. Seed
those known extents before applying the requested offset; leave unknown rows
estimated and leave the lazy-builder path unchanged. Incular uses a stable
sized row host to model Flutter's stateful host, and enables native semantics
on the row's contents explicitly.

The three focused keep-alive regression targets passed.
`scroll_viewport_lists` had the same 61 passes and 38 failures before and
after this change, with identical assertion locations and values for those
failures. This file does not claim a clean result for that target.
