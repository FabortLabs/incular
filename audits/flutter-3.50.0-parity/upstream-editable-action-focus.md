# EditableText completion and keyboard focus

Pinned Flutter 3.50.0 `packages/flutter/test/widgets/editable_text_test.dart`
registrations 4029, 4064, 4104, 4314, and 4393 all passed.
The corresponding native tests mount an editor with a real FocusNode,
deliver the native action, and inspect both callback order and input
client commands:

- Explicit completion leaves focus in place. Completion runs before
  submission, and submission receives the committed editing value.
  The original at 4104 says "next" but actually sends Done. Its tested
  path is the same Done path; a separate native check also sends Next.
- A normal Done closes the old attachment without reopening it.
- If onSubmit requests focus on the same editor, the old client is
  cleared and a new client is attached and shown in the same input
  turn. No intermediate Hide is sent. An Unspecified action leaves
  the existing client attached.

An independent native check confirms that None clears composing text
without moving focus. Unspecified still calls submission without
restarting the keyboard. The native suite ran 46 focused tests,
including existing input-gating, lifecycle, and IME regressions.
This verifies the native command boundary, not physical keyboard
visibility on a particular handset.

## Newline and selection-only updates

Pinned originals 4144, 4184, and 4445 also passed. Mounted native
tests show that a single-line Newline invokes completion before
submission, a multiline Newline invokes neither callback, and a
selection-only native update moves the caret without emitting a
Show or SetClient command. All ten tests in the focused native
completion suite passed.
