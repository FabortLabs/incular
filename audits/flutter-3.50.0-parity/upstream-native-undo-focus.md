# Native undo when focus moves

The pinned `services/undo_manager_test.dart` dispatch and channel-error
registrations (lines 15 and 49) passed on both sides. A Flutter-compatible
embedder can register its current undo client, dispatch the same JSON
undo and redo messages, and send the two state flags over
`flutter/undomanager`. An asynchronous transport failure emits exactly
one report containing the original error, the services library, and
the `UndoManager.setUndoState` delivery context. The reporter is scoped
to the Rust service; the caller also receives the typed error.
The native editor can continue using typed platform events.

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
hardware or native iOS undo manager adapter is asserted by this
runtime test. Channel failures are verified independently as described
above. This authored scenario
does not add to the upstream registration count.
