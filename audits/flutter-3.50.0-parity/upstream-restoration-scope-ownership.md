# Restoration scope ownership

Five pinned `services/restoration_bucket_test.dart` cases passed.
The native restoration tests use the real manager and persistence
store, not name or declaration matching.

## Same-turn handoffs

The original :237 and :275 now pair. Incular's ordinary `claim_scope`
still rejects duplicate live paths. For a deliberate replacement,
`prepare_scope_handoff` creates a fresh private scope. It cannot read the
incumbent's values, and its writes neither mutate the incumbent nor
trigger a save. Once the previous lease is released, a single
replacement can atomically commit; the resulting lease keeps ownership.
The new value survives application restart, the old value does not,
and the similarly named sibling stays untouched. A second pending
replacement prevents both from committing and publishes no snapshot.
Dropping the duplicate permits the surviving replacement to commit.

The native controls also ensure use of the normal claim during a
pending handoff is rejected, live descendants prevent deletion, and
reset invalidates an uncommitted replacement. These are supplementary
native safety controls, not new upstream registrations.

:200 is partial: finalizing while the previous owner is active fails
and leaves persisted data unchanged. Flutter additionally checks the
two debug-owner names and a formatted parent identifier; the native
typed error does not carry those labels. :13 is partial: native writes,
overwrite, deletion, absence and persisted null are tested, but the
raw map format and Flutter diagnostic owner do not match Incular's.
:291's exact original passed; the explicit-disposal comparison is
separate from the ordinary lease release, which intentionally retains
data.

All 36 focused native cases passed.
