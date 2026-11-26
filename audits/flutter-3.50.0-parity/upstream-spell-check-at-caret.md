# EditableText suggestion lookup at the caret

Four pinned EditableText registrations pass in Flutter and their corresponding
queries pass against an actual mounted Incular editor receiving suggestions
through its spell-check service. The lookup includes both word boundaries
and returns no suggestion before the first span, between spans, after the
last span, or while checking is disabled. Incular uses its usual UTF-8 byte
offsets; the paired upstream fixtures are ASCII.

A separate Flutter probe and Rust regression pin the tie when two spans
share an endpoint: Flutter uses the lower midpoint first. The old Incular
lookup returned the right span for two touching spans. This authored case
is not counted among upstream registrations. This evidence covers lookup,
not opening or styling a suggestion toolbar.
