# EditableText password spell check

Six controls ran against pinned Flutter 3.50.0 and matched the retained
Incular editor with an instrumented spell-check service. Spell check stays
disabled for obscured text, a visible-password keyboard, a numeric-password
keyboard, and password autofill hints. Removing obscuring from an ordinary
editor checks the current text once; removing it from a visible-password
editor does not enable checking.

The Incular tests additionally check a secondary new-password hint, a
username control, and an in-flight response delivered after suppression.
They assert the service receives no suppressed text, not just that the
widget draws no underline. Old suggestions are cleared while suppressed.
No claims about native device keyboards or browser spell-check UI are made.
