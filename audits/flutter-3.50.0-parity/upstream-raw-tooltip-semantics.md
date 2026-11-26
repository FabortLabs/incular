# RawTooltip anchor semantics

Pinned Flutter 3.50.0 `widgets/raw_tooltip_test.dart:1539` and
`:1597` passed. Native runtime fixtures inspect the retained screen-reader
tree. Before the fix the child was labeled "Bar" and a separate
ancestor carried "Foo" as a tooltip. The anchor's own text node now
has label "Bar", tooltip "Foo", and the ambient left-to-right
direction. Clearing the annotation leaves the anchor label and
direction intact and still permits the overlay to show. These tests
do not validate native screen-reader speech, announcement-channel
events or every overlay traversal hierarchy.

All 24 focused native cases passed, including tooltip animation and
positioning regressions.
