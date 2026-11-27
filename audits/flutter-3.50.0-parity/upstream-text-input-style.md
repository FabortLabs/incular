# Text input style metadata

Five pinned services originals pair. A text-input style compares all
eight attributes by value, and equal styles hash equally. The JSON
uses the exact Flutter field names and indices, including bold 6,
center 2 and left-to-right 1. Missing optional attributes remain
explicit null fields, not absent keys or invented defaults.

The outbound `TextInput.setStyle` call uses `flutter/textinput`.
When the host rejects it, the reporting adapter delivers one typed
platform exception preserving “Failed to set style” and the context
“while updating text input style.” A separate control rejects
nonfinite numbers before emitting a message. Six focused native
style tests and four nearby event-channel tests passed. This covers
the style API and wire adapter, not a claim that native adapters
automatically use every style field.
