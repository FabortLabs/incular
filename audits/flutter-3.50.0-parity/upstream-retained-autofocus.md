# Mounted Focus lookup and autofocus

Pinned source: `widgets/focus_scope.dart` (`_handleAutofocus` and
`didUpdateWidget`), `widgets/focus_manager.dart` (`_Autofocus.applyIfValid`),
and `widgets/focus_scope_test.dart`.

- **955:** changing `autofocus` from false to true on the same mounted Focus
  and FocusNode now requests primary focus. The Incular regression failed
  before the change: only initial layout and new mount handling saw the flag.
- **969:** inserting an autofocus node does not displace the already-focused
  node in the same scope.
- **1213:** a child resolves the closest Focus node and programmatically
  focuses it; the mounted wrapper receives exactly one true callback.

Two authored Flutter references exercise the one-shot guard after unfocus
and a toggle inside an empty nested scope while a separate scope owns
primary focus. The Incular integration tests assert both outcomes. Runtime
processes the changed retained node after layout and verifies the original
scope and attachment before applying the request. No steady-frame tree scan
was added; a stable view has no pending updates. Existing deferred initial
layout, editor focus, and scope-history suites were also run.

## Partial matches

- **941:** a real retained top-level FocusScope can receive focus; Incular
  still uses an explicit scope in the fixture and does not expose Flutter's
  implicit view/traversal scope ancestry under the FocusManager root.
- **1105:** Focus lookup excludes the starting element, stops at the nearest
  scope (unless permitted), and is independent of semantics. The retained
  lookup does not subscribe a BuildContext and is not an assertion of
  the exact enclosing FocusManager tree depth in Flutter's test harness.

The change does not extend autofocus to every new subtree mounted after
startup, change the existing initial frame policy, or settle the ordering
of multiple concurrent autofocus requests in different scopes.

Native: `flutter350_focus_root_and_autofocus`, `initial_focus`,
`flutter350_owned_focus_scopes`, and `flutter350_editable_focus_node` in
`incular-runtime`. Flutter probe: `retained_focus_autofocus_test.dart`.
