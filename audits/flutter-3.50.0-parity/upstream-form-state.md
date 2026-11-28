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

## Validation on focus loss

Four exact registrations in the pinned `widgets/form_test.dart`
passed. The native Runtime fixture uses real, undecorated
EditableText fields and checks the mounted error labels:

- :1597: typing in the first field does not show an error. Moving
  focus validates only the field just left, including an untouched
  second field when it loses focus later.
- :1657: a field set to Always validates immediately inside a form
  set to OnUnfocus; its sibling still waits for its own blur.
- :1719: the platform Next editing action focuses the second editor.
  The native control also verifies that this blurs the first field.
- :1803: explicit form validation shows errors on all registered
  fields, including the currently focused field.

A non-traversable focus boundary observes descendant focus without
becoming an editor or adding a stop to keyboard traversal. Typed
field builders now update from their own value and error revisions,
without reconstructing the whole form. All 37 focused native form
and editor-focus tests passed. This does not cover form-wide
Always scheduling or assistive announcements.

## Conditional and deferred automatic validation

Four exact pinned registrations passed. At :929 and :969,
`OnUserInteractionIfError` does not validate an initially valid
or empty field just because it built. Explicit validation of the
empty field shows the mounted error. The :1180 neutral fixture
types, clears, and retypes using the actual focused editor.
The form is silent while no error exists, manual validation shows
the error, and the next valid edit clears it. Becoming invalid
again after the previous error clears stays silent.

At :1767, form-wide Always schedules validation after the first
paint: neither error is shown on frame one, both are mounted on
frame two, and the runtime requests that follow-up frame. The
native control also reconfigures an existing field from conditional
validation to Disabled and then back to the controller's policy.
All 42 focused native cases passed.
