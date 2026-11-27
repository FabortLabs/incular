# Neutral form state, validation, and editing

Eight exact Flutter `widgets/form_test.dart` registrations passed.
Incular exercises the form contract with neutral widgets and an
actual focused EditableText, not Material decoration.

- :14 saves the current text before another frame, including an
  empty string after a real selection and delete.
- :95 restores the field's initial value, clears presentation and
  interaction state, and invokes its reset callback once.
- :237 and :282 query real validators. The invalid result does not
  set an error, and a later valid result does not clear a stale
  presentation error; explicit validation does that.
- :330 validates all three registered fields and returns only the
  distinct IDs of the two invalid fields.
- :1295, :1317 and :1344 track actual edits. Focus alone and
  programmatic assignment are not user edits, typing is, and
  resetting the field clears the flag.

The 24 focused native editor and form cases passed. The adjacent
restoration test's serialized selection expectation includes the
existing affinity and direction metadata. No Material styling or
accessibility announcement parity is claimed.

## Forced and explicitly cleared errors

Six further exact pinned registrations passed. At :1375, :1401,
:1438 and :1471, a forced error shows from first layout or from a
configuration change, makes form validation fail, and bypasses the
validator. Reset clears the presentation without removing the force;
the next explicit validation restores it. The native control also
removes the forcing option on the same registered field.

At :1975 and :2002, clearing one field or the entire form removes the
rendered error labels without changing the values. The native fixtures
read mounted text widgets after each update. All 26 focused native
form state and error-presentation cases passed.
