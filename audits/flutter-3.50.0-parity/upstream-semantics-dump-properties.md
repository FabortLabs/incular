# Semantic validation and custom action diagnostics

Pinned `semantics/semantics_test.dart:831,848` passed. The diagnostic
now reports whether a collected node is explicitly valid or invalid,
and whether it has empty bounds. The native control verifies that
the existing legacy invalid flag takes precedence and that an
unspecified result produces no validation detail.

The custom action marker now includes the actual registered labels
in order. The native control uses the original three labels and
also verifies bounded output stops within the label list without
changing the retained tree revision. Overrides provide a hint for
an existing action, not a new custom menu entry. Both native tests
failed before the richer diagnostics and all 20 focused semantics
and custom-action tests passed after.

Pinned :492 (the detached node dump without a transform) and :807
(blocked user-action markers) also passed. Incular stores final
rectangles rather than detached transform metadata, and its
portable node removes blocked user actions instead of retaining
separate debug marks for them. Those exact diagnostic behaviors
have not been credited.
