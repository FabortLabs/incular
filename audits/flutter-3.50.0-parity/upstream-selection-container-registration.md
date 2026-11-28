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

All seven pinned originals are now paired. The additional native run
drives both explicit registrars and inherited `SelectionRegistrarScope`
instances: three selectable children publish one container; an empty or
disabled container publishes none; and swapping a delegate keeps the
parent and external registrations current in the same layout pass. An
explicit registrar takes precedence, can return to the inherited
registrar, and releases its registration when the tree is dropped.

Nine native registration tests and seven adjacent selection lifetime
tests passed. This covers container registry membership, not arbitrary
delegate hooks or forwarding selection events through nested proxies.
