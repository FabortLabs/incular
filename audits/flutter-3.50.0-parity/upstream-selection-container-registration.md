# Mounted SelectionContainer registration

Compared Flutter 3.50.0 `widgets/selection_container_test.dart` to the
actual retained tree in `flutter350_selection_container_registration.rs`.

- A nonempty enabled child contributes exactly one entry to its nearest
  accepting registrar, irrespective of its own paragraph count. Parent
  counts and content flags are resolved in the same layout pass.
- Disabling a child omits both the proxy and all its selectable leaves
  from the enclosing registry, without dropping their paint.
- Replacing a delegate in place preserves the retained element and
  enclosing registration, clears the detached delegate, and resolves
  the replacement's content without waiting for another layout.
- Empty, enabled, and empty-again transitions are tested across three
  nested registrars. An independent nested SelectionArea remains
  independent, as tested by the adjacent selection lifetime suite.
- A mounted zero-area SelectionContainer lays out without crashing.

All seven pinned originals passed. All five native cases and seven
adjacent selection lifetime tests passed. Line 202 is paired. Lines
14, 40, 65, 117, and 147 are partial: the widget registration and
delegate state match, but the originals also exercise a separately
injected registrar. Line 179 is partial: the retained tree infers the
parent boundary but has no public SelectionRegistrarScope injection
surface yet. This slice does not forward selection events through
nested proxy containers.
