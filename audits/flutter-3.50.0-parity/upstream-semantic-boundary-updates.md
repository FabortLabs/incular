# Updating between semantic boundaries

Pinned `semantics/semantics_test.dart:408` passed. A real mounted
native boundary and three intervening annotations now publish their
four disjoint actions on one node. The nested boundary retains its
own scroll-up action. Replacing the middle scroll-left handler with
scroll-down and restoring it keeps both semantic IDs; dispatch
delivers the current actions, rejects the stale one, and leaves the
inner boundary separate.

Before the fix, an action-bearing boundary refused to absorb even
compatible argumentless actions with no text. Compatible annotations
can now contribute actions without requiring a label, and a nested
boundary terminates its own branch without vetoing other fragments.
Conflicting actions still remain separate. All 45 focused runtime
handler, sliver, pointer-blocker and semantic text tests passed.

Pinned `:322` remains partial. In Rust the committed child vector is
owned by the retained node. Mutating the caller's vector, by growing
it or replacing its contents at the same length, does not alter
the existing list or its revision. The new vector takes effect only
on an explicit update. Flutter instead diagnoses mutation of its
shared list at run time with error hints and child descriptions.
