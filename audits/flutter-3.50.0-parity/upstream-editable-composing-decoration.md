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
