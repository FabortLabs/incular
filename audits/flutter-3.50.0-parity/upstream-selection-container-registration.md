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
delegate hooks; enabled proxy forwarding is tested below.

## Nested selection commands

The authored Flutter control `selection_container_select_all_test.dart`
verifies that Select All returns the first, nested second, and third
paragraphs in document order, but not text inside a disabled container.
The inner delegate independently reports only the second paragraph.
The mounted Incular probe first reproduced the dropped nested text
(`firstthird`) and now passes with the same selected text. A reverse
selection across the proxy preserves the local backward range (6
to 0), and clearing it empties both snapshots without losing the
child's registered content. Independent SelectionArea behavior
continues to pass the native lifetime suite.

The authored Flutter probe and all 17 targeted native tests passed.
No additional original registrations are counted by this slice.

## Custom selected content

The pinned `selectable_region_test.dart` case
`Can extend StaticSelectionContainerDelegate` and the native mounted
equivalent both select from byte-equivalent caret positions 4 and 10 in
two rendered paragraphs. The child's custom combiner produces
`o World!\nHow are yo`; the enclosing region publishes that exact
string to its selection-change callback. Nested formatters compose
per direct child rather than flattening all leaf strings, and absent
selection does not invoke a formatter. Formatting does not change
the underlying range, highlights, or caret geometry. All 19 focused
native tests passed.
