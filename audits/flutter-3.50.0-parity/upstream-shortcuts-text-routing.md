# Shortcuts in front of a focused editor

Reference: Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`.

Native `WidgetsApp` now supplies the two stock no-op intents. The ordinary
one consumes the event; the other stops its trip through framework
ancestors but returns it to the host. An explicit replacement of the
app action map still replaces the defaults. A configured no-op can also
shadow an arbitrary command from a nearer action scope.

Mounted native tests send real key-down/up events to an autofocus
`EditableText`. They cover a miss in an inner map, a mapped key without
an action, a consuming action, a nearer nonconsuming action, and both
stock no-ops. Only the cases passed to the host accept the subsequent
native text-input event; none moves primary focus.

All six pinned originals (865, 908, 953, 980, 1018, 1058), the seven
focused native editor routing tests, and seven adjacent dispatcher and
app activation tests passed. The separate app navigation-focus target
had four failures even with the new no-op wrappers removed; that
pre-existing issue is tracked independently.
