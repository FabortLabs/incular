# Semantic validation and custom action diagnostics

Pinned `semantics/semantics_test.dart:831,848` passed. The diagnostic
now reports whether a collected node is explicitly valid or invalid,
and whether it has empty bounds. The native control verifies that
the existing legacy invalid flag takes precedence and that an
unspecified result produces no validation detail.

The custom action marker now includes the actual registered labels
in order. The native control uses the original three labels and
also verifies bounded output stops within the label list without
changing the retained tree revision. Overrides provide a hint for
an existing action, not a new custom menu entry. Both native tests
failed before the richer diagnostics and all 20 focused semantics
and custom-action tests passed after.

Pinned :492 (the detached node dump without a transform) and :807
(blocked user-action markers) also passed. Incular stores final
rectangles rather than detached transform metadata, and its
portable node removes blocked user actions instead of retaining
separate debug marks for them. Those exact diagnostic behaviors
have not been credited.

## Identity at the Flutter allocation limit

Pinned `semantics/semantics_test.dart:928` passed. It keeps 65,535
framework IDs live, removes ID 1400, and expects the next object to
reuse that exact numeric ID. The native test holds the same number of
live nodes and reuses the retired node's storage slot. The generational
identity changes deliberately: neither a lookup, an update, nor
another removal through the old ID reaches the replacement. Incremental
observers see the replacement only. The adjacent AccessKit test confirms
that a delayed action for the old native ID cannot hit the replacement.
Both focused native checks passed. This is partial: Incular does not
reproduce the Flutter engine's 16-bit numeric reuse contract.

## Selectable child order in semantic dumps

Pinned `semantics/semantics_test.dart:580` passed. The additional pinned
`semantic_dump_child_order_test.dart` exercises two overlapping nodes
whose explicit traversal keys reverse their paint order. Default
diagnostics list traversal order; the inverse hit-test variant lists
paint order.

Incular exposes the two orders through the mounted tree and
application diagnostics. It reuses the permutation already used by
point dispatch, without storing a second tree or changing what native
adapters receive. The native test verifies both orderings at the root
and within a semantic container, including changing sort keys, actually
swapping paint order, and dispatching the resulting topmost action.
The three adjacent rendering pipeline checks passed as well. Text
format and generational identifiers are native, not Flutter's dump
syntax.
