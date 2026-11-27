# Focus outlines are diagnostics, not default decoration

Pinned `widgets/debug_test.dart:336,389,403,450` passed. The native
runtime matches all five Flutter debug states: green primary focus,
blue focused ancestor, cyan focusable, yellow skipped for traversal,
and red unfocusable. Each foreground outline is 3 logical pixels.
Focus and permission changes update an existing wrapper, including
retaining the green primary color when traversal is disabled.

The renderer emits no border on a regular Focus widget. Enabling
the per-runtime debug option adds a retained foreground picture,
and disabling it removes that picture even when the rest of the
scene is unchanged. A separate EditableText check still emits no
unrequested border when focused. The diagnostics are off by default
and inert in release builds. They do not change layout, focus, or
semantic annotations.
