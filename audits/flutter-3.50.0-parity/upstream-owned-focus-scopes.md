# Widgets: default FocusScope ownership

The pinned Flutter 3.50.0 default `FocusScope` creates a focus scope
node, even with no explicit node or property overrides. Incular used
to attach only a traversal policy to the child in that case. That
lost scope identity, an independent focused-child history and a scope
boundary for lookup. The widget now instantiates one owned node on
mount and retains it on compatible updates.

A neutral Flutter probe and its runtime twin verify sibling scopes
have distinct nodes, share the same enclosing scope, keep their own
child focus histories, and preserve focus through descriptor rebuilds.
A direct reparenting check keeps the same node/element focused with no
transient blur; removing both external and owned scopes detaches them
and clears their hosted histories. Removing only the focused child
does not arbitrarily focus its surviving sibling.

The original `widgets/focus_scope_test.dart:563` scenario is paired.
`:394` and `:611` are partial: the tested reparenting and detachment
assertions agree, but the exact global Flutter root-scope counts or
`setFirstFocus` calls are not compared. All three Flutter originals
and the authored reference passed. Four dedicated Rust regressions
and 36 nearby focus policy, lifecycle, and traversal tests passed.
Five strict frame controls still match: default scope traversal,
arrow focus, reverse Tab, retained EditableText initial selection,
and cross-fade sizing.
