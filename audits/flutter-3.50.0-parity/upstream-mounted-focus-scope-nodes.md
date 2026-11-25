# Mounted external focus scope nodes

The pinned Flutter originals `focus_manager_test.dart:1607` and
`focus_scope_test.dart:1085` passed by exact name. A mounted external
scope now has its own retained node. Child focus marks the scope as
focused without giving it primary focus; an explicit
`request_scope_focus()` takes primary focus for the scope rather than
following its previously active descendant. The scope anchor is not
an ordinary Tab stop. The Rust runtime tests use the nested focus
fixture and also request focus on a centered scope measured at exactly
zero width and height.

This change does not infer the nearest scope from a build context or
implement Flutter's complete per-scope focused-child history.
