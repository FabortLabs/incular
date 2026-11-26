# Native undo when focus moves

The pinned `services/undo_manager_test.dart` dispatch and channel-error
registrations (lines 15 and 49) both passed. Incular's editor receives native
undo and redo through typed platform events. It does not yet expose an
arbitrary client registered on Flutter's JSON channel or report a failing
outbound channel through a framework error callback. Those gaps remain
separate from the fixed editor behavior.

The paired authored reference `native_undo_focus_handoff_test.dart` seeded
two independent editors, changed both, then requested focus for the first.
After Flutter processed focus microtasks but before a new layout frame,
its native undo message changed the first editor from `two` to `one` and
left the second at `beta`. It sent `canUndo: false, canRedo: true` for the
first; a redo aimed at the second did not consume the first's redo.

The Incular regression uses two actual focus nodes and checks both
histories have undo available. Before the fix, native undo read the last
rendered focus and changed the outgoing editor from `beta` to `alpha`.
It now adopts pending focus on native input just as it does on a keyboard
dispatch. The native test verifies both text values, the reported iOS
undo state, and the symmetric focus handoff for redo. No physical iOS
hardware, native iOS undo manager adapter, or Flutter channel failure
handling is asserted by this native runtime test. This authored scenario
does not add to the upstream registration count.
