# Semantic input kind and controlled-node identifiers

All three pinned `widgets/basic_test.dart` cases passed. Lines **557** and
**568** have an identical reporter name; both original registrations were
selected. They preserve the singleton `abc` and merge a nested relation
into the union `abc, ghi, def` without duplicates. An authored paired
probe rebuilds only the absorbed child, keeps the same parent semantics
identity, and removes the stale identifier. Native controls also verify
that explicit child nodes and accessible labels stay available.

Line **583** retains a declarative phone input kind without making an
otherwise inert node editable. Editable controls derive the semantic
kind from the same keyboard category as Flutter; a semantic override
does not change the editor's keyboard. The AccessKit projection keeps
phone, URL, email and search fields distinct while preserving the
password role for an obscured field.

Native: 26 focused runtime tests (input/control, role boundaries and
semantics binding) and one exact AccessKit projection test passed.
Controls are now retained as identifiers in the portable tree; these
fixtures mount no targets and do not establish browser or native
target-ID resolution.
