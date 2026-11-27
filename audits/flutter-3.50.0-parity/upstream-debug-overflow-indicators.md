# Debug overflow indicators

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
References: `rendering/debug_overflow_indicator_test.dart`,
`rendering/flex_overflow_test.dart`, and
`lib/src/rendering/{debug_overflow_indicator,flex,shifted_box}.dart`.

Flex and unclipped constraints-transform renders now record the debug
indicator only while a nonempty render box actually overflows. The local
markers cover the outer 10% of each affected edge and use Flutter's
repeating black/yellow diagonal stops. Labels are drawn on white over the
stripes. The retained diagnostic picture is allocated only for overflow,
above children and clips. It disappears when the child shrinks or the
constraints transform is clipped; fitted and zero-size boxes never allocate
the diagnostic picture. The implementation is compiled only in debug mode.

The first painted overflow emits its diagnostic once per render object;
repeated frames and recovery followed by another overflow do not spam the
report. Constraint reports expose summary severity and the physical
overflow amounts. The native check verifies the pinned centered 200-in-100
fixture: 50 pixels above and 50 below, with two edge indicators. The Flex
fixture verifies the fitting, overflowing, and zero-height states.

All four exact pinned originals passed. Six focused native indicator cases,
nine adjacent constraints-transform cases, and seven Flex clip/reporting
cases passed. This verifies resolved display-list geometry, style inputs,
stacking and reporting, not a GPU raster golden or Flutter hot-reload's
reassemble-time report reset.
