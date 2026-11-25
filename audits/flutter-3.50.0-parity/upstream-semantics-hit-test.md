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
