# Focus widget semantics

## Pinning the initial behavior

Five exact-name Flutter `widgets/focus_scope_test.dart` registrations
passed on the pinned reference. The Runtime target
`flutter350_focus_widget_semantics` exposes two current failures:

- :1358 Keyboard focus itself is gained and retired correctly, but the
  retained label never reports that it is focusable or focused. It should
  start focusable/unfocused, become focused, and withdraw the flags when
  the same node is no longer eligible.
- :1846 A single RTL text label preserves its semantic identity and
  direction. However, it has no Focus action and dispatching a focus
  request against its semantic node does not move keyboard focus to the
  surrounding Focus widget. Flutter does both. The Rust test checks
  the actual action dispatch, retained node and input focus, not just API names.

The test for :1886 verifies that the mounted Focus widget measures to
zero under a zero-area view and can then process a focus request without
a panic. It passes.

:1633 and :1085 are reference-only for now. Incular's Focus has no
includeSemantics builder, and its FocusScope cannot be supplied the
external FocusScopeNode the original uses. An absent annotation by default
or a different no-crash widget would not cover those behaviors.
