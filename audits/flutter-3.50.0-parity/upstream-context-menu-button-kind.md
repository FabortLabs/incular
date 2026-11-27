# Context menu button kinds

All seven pinned `widgets/context_menu_button_item_test.dart` registrations
are paired. The canonical kind defaults to Custom and converts to and from
all ten legacy types. Translate remains a distinct kind, but reports the
legacy Custom type. An explicit label still takes precedence.

A legacy and newer representation of the same action compare and hash
identically. Copies accept a new kind or a converted legacy type and
retain unchanged fields and callback identity. Constant patterns work
with the new non-exhaustive kind. This adds an action vocabulary, not
a native translation service; the caller supplies the action.

The legacy field has become a derived `button_type()` getter so it
cannot disagree with `kind()`. The changelog provides migration steps.
Seven widget tests and eighteen runtime menu and clipboard regressions passed.
