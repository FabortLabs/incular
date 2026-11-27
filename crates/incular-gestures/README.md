# incular-gestures

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Pointer recognition, gesture arbitration, keyboard and focus mechanisms. |
| API class | application; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Available; shared observation contracts consolidate in C/G. |

Platform-neutral pointer recognizers and interaction state for Incular.

This crate owns pointer event values, a pending/accept/reject/cancel gesture
arena, tap/drag/scale recognizers, pointer hover tracking, focus nodes, and
keyboard shortcut dispatch. It depends only on
`incular-core`; retained widget regions and native event loops adapt these
primitives rather than the other way around.

`GestureDetector::observe` selects an action without calling application code;
the retained `GestureDetector` adapter uses that boundary to wait for
`GestureArena` arbitration.
Direct `GestureDetector::handle` remains available for standalone use. Arena
keys include both window and pointer identity. Scale members explicitly opt
into simultaneous compatibility, while tap, long-press, pan, and directional
drag members remain exclusive and receive `GestureCallbacks::on_cancel` when
they lose.

## Focus-highlight notifications

`FocusHighlightManager` commits modality changes before notifying listeners.
Callbacks may read the manager, change strategy, or register/drop subscriptions
without borrowing its internal state. Reentrant changes update the current mode
immediately; notifications are delivered synchronously in commit order after the
current event, not recursively. Event values describe committed transitions;
queries return the latest state. Newly registered listeners start with a later
commit, and dropping a subscription cancels even its queued notifications.

Unchanged input/mode creates no notification. Pending delivery holds weak
registrations and no persistent event history. Application transition sequences
must converge. A callback panic propagates, leaves committed state intact and
discards unfinished delivery; later changes remain usable. This contract concerns
modality, not focus-scope selection transactions.

Use `SingleActivator::with_num_lock` to require a locked or unlocked
Num Lock state. The default is `LockState::Ignored`. Mounted
`CallbackShortcuts` receive the runtime's separate lock snapshot.

A standalone `LongPressGestureRecognizer` can use an external arena.
The hold still waits for its deadline and an arena award; disposing a
pending hold withdraws that member. In `GestureArenaManager`, a survivor
then resolves at the deferred-resolution boundary, not during disposal.

`MonodragGestureRecognizer` defaults to the most recently accepted contact.
Use `with_multitouch_drag_strategy` to sum contacts or compute directional
boundaries for axis drags and average free pans. A host using frame-based
averaging supplies the current system-frame timestamp through
`set_multitouch_frame_time`; pointer timestamps do not reset the frame.
`OffsetPair` carries local and global offsets together. Construct it from a
raw event's position or delta to preserve the two spaces.

Use `VelocityTracker::try_add_position` and `try_velocity_at` to estimate a
single pointer's velocity independently from a recognizer. Checked samples
must have nondecreasing timestamps; the platform fling estimators require
at least one sample.
