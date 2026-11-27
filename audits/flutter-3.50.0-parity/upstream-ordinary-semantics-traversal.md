# Traversal order in ordinary widget trees

Pinned `widgets/semantics_test.dart:1239,1304,1343,1385,1432` all passed.
The native `flutter350_semantic_traversal_order` suite verifies the
unkeyed column/row and quarter-turned row cases, including each text
label's reading direction. A subsequent unchanged frame does not send
another native accessibility projection.

The no-direction original uses six nodes with indistinguishable button
flags. The authored pinned `semantics_no_direction_order_test.dart`
uses identifiers on the same stack and actual traversal output. It
confirms paint order in both axes when no direction is inherited, as
the pinned traversal implementation specifies. Incular matches the
observable identifier order.

## Typed custom sort keys

The two explicit numeric tests (:1239, :1304) now use a distinct custom
numeric ordering key for the Dart `CustomSortKey` groups, and the nested
test uses ordinary ordinal keys in its inner group. Both matching trees
are verified on Incular. The authored pinned
`semantics_custom_sort_key_test.dart` and its native twin add two custom
ordering types, an ordinary ordinal type, adjacent different names,
descending order and a tie. A type or name boundary prevents comparisons
across the boundary, and equal keys remain stable.

Incular models extension with a Rust ordering type rather than a Dart
subclass. The key is included in retained annotation metadata and kept
on the outer node when a viewport splits its native semantics panes.
All 35 selected runtime regressions passed.
