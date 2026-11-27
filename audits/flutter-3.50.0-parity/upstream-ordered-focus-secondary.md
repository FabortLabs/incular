# Ordered focus: secondary policy and nested groups

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Reference: the five selected OrderedTraversalPolicy originals from
`widgets/focus_traversal_test.dart`.

The default secondary sort remains reading order. A group or scope can
now opt into widget order for equal or absent explicit orders, matching
the selected Flutter fixtures. Directionality affects the reading
fallback, not registration order. The authored comparison separates
these possibilities with fixed-position RTL rectangles, where the
original same-direction Row by itself would not distinguish them.

Nested groups now carry their own focus order. The parent sorts groups
by that order, not by a descendant's numeric or lexical value. Inside the
group, its own policy still decides its members' order. The previous
path began at node 4 instead of node 9 in the exact nested fixture.

All five pinned originals, four authored Flutter secondary-policy
variants, and six native ordered tests passed. Numeric and lexical
cases replace previous partial evidence with full direct mounted node
navigation and both cycling directions. The focused adjacent reading,
sequential, and general traversal tests also passed. Arbitrary custom
secondary policies are not implemented by this change.
