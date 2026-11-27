# Semantics: hit-test behavior metadata

The pinned `semantics_test.dart:1078` and `:1093` originals pair
the default (defer), opaque, and transparent choices and their
persistence when a configuration is copied. Incular exposes the
field independently from pointer hit-testing, and the retained
node preserves it together with the semantic boundary and label.

The authored Flutter probe and native runtime twin verify that an
explicit parent wins and otherwise the first merged descendant
with an override supplies the value. Updating that property retains
the node identity. Two targeted runtime tests and eight nearby
framework semantics checks passed. This checks the framework data;
it does not change or verify a platform overlay's pointer routing.

## Explicit Web pointer override

A Web semantics overlay now assigns `pointer-events:all` for an explicit
opaque policy and `none` for transparent. Changing only the policy on a
retained node rewrites its style even if the position is unchanged.
Defer retains Incular's existing canvas pointer routing; the pin's
additional inferred browser policy is not claimed here.

The native projection test proves every policy arrives on the same DOM
node's native ID at unchanged geometry. The actual Web crate checked
successfully for wasm32, serially. This was not a browser-driven
pointer or assistive-technology test.

## Point actions use inverse paint order, not reading order

Two authored Flutter probes and the matching native
`flutter350_semantic_point_paint_order` fixture overlap two accessible controls
both at the root and inside an explicit semantic container. Reading order
can put the visually front control first, but dispatching a tap by position
must still choose the front control. If it has no handler, the control
behind it receives the tap. Reversing the keyed paint order changes the hit
without moving either control or changing traversal. The right edge is
excluded at device pixel ratio 1.

Before the fix Incular walked the reversed **reading** order and selected the
back control. The runtime now retains a separate sibling order only when
the reading sort changes the original order. It clears that order when it
matches again or the parent/accessibility tree is retired. This checks the
runtime's logical point dispatch; it does not imply a separate paint-order
API in every native projection, or change special synthetic pane grouping.
The 19 selected regressions passed. This is an authored comparison and
adds no original registration credit.

## Default focus blocking is not an annotation

Pinned `semantics_test.dart:1064` passed. A blank native annotation
and one explicitly assigned the default focus-block value both
produce no semantics node. Switching to BlockSubtree records the
new value and creates one nonfocusable annotation; restoring None
removes it. The native fixture uses the retained tree rather than
only checking an enum default. Two direct and 24 adjacent cases pass.
