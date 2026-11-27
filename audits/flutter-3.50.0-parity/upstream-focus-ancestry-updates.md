# Focus ancestry, sibling removal, and scope history

Five exact pinned `focus_manager_test.dart` registrations passed.
The native mounted cases pair :23, :46, :547 and :592. Appending
or removing focus nodes updates both sides of the parent link and
preserves the surviving sibling's identity. An ordinary parent
receives descendant focus without becoming primary, and the
enclosing scope tracks the active leaf. Removing the middle
ancestor drops its grandchild from scope history.

For :518, an authored Flutter probe separates the two phases.
Immediately after explicit `FocusAttachment.detach`, the old
leaf and scope have no focus and history is empty. After pumping,
the empty scope itself becomes primary. Incular's mounted update
matches the completed frame and does not focus the sibling; this
does not establish a match for Flutter's separately observable
pre-frame `FocusAttachment.detach` call. The ledger leaves :518
partial. Five focused and seven adjacent native tests passed.
