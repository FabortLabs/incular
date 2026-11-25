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
