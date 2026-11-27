# EditableText composing underline

The original pinned `widgets/editable_text_test.dart:6847` passed.
It initializes the composing range 5..14 in “text composing text”,
focuses the editor, and then unfocuses it. The mounted native test
checks the single underline's actual display-list rectangle against
the shaping engine's 5..14 selection geometry. That rectangle is
gone after blur without losing glyphs or changing the committed text.
The decoration is a text run, not a field border.

The authored `editable_composing_visibility_test.dart` also confirms
that a read-only, obscured, or unfocused editor does not underline that
range. Native coverage checks a host IME preedit containing multibyte
text: it is underlined before commit, does not alter the committed
controller value during the preview, and stops underlining after commit.
Existing outside-tap and editing-action focus tests still pass.

## Combining marks and first-frame autofocus

The pinned `widgets/editable_text_test.dart:18461` sends a composing
range covering only the combining acute accent in `Á`. Its range is
UTF-16 1..2 in Flutter and UTF-8 bytes 1..3 in Incular. The native
mounted frame preserves both editing ranges and paints successfully.

The pinned :11156 fixture starts with four text lines, right alignment,
a ten-line maximum, and an upstream caret at byte zero before
autofocus. The retained native frame keeps that selection and affinity,
paints the text, and resolves its selection-handle geometry without
throwing. No design-system handle appearance is being compared.
