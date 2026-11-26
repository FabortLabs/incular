# EditableText keyboard language hints

The pinned `hintLocales is sent to the engine` case succeeds. Incular's
mounted editor attaches its input client with the same ordered `en`,
`fr` preferences. Its Flutter message encoder puts the BCP-47 list on
both the triggering editor and its registered autofill entry, including
a script and region tag. An absent preference is JSON null, and an
explicit empty list remains an empty array.

Changing or clearing hints on a focused field reconfigures the existing
editor without moving focus or issuing a keyboard-show command. The
Android bridge retains the same distinction from a nullable JNI string
to `EditorInfo.hintLocales` on API 24 and later. Tests covered the
mounted native command and Flutter wire message, not a device keyboard.
