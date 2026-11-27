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

The two explicit numeric tests have the same sorted sibling and nested
orders on Incular. Their use of a distinct Dart `CustomSortKey` subclass
is not yet represented by the Rust ordinal key exercised here, and
remains partial until custom type compatibility is tested.
