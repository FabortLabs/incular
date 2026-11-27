# Sequential focus invoked from a particular node

Four exact Flutter originals in the `WidgetOrderTraversalPolicy`
group passed. A mounted node's invocation uses its nearest focus
scope, not the window's unrelated primary target or the invoking
node as the current cursor. A fresh scope enters on the first
forward or last reverse focus stop. A skipped wrapper can enter
its scope and focus the inner widget, with or without an extra
padding wrapper or a difference in geometry.

The native implementation exposes this through synchronous
`Runtime::next_focus_from` and `Runtime::previous_focus_from`, reusing
scroll-to-reveal and scope edge policy. The extra authored pinned
probe confirms a dormant scope moves from its remembered child,
not from the unrelated primary focus. A native control also
rejects a detached origin without changing focus. All 48 focused
Rust focus/scroll regressions passed.
