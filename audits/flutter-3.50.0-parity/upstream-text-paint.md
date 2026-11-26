# Solid text paint precedence

Pinned `painting/text_style_test.dart:506` passed. All original solid
foreground combinations now have an equivalent visible result. Plain
colors blend continuously. If either endpoint supplies foreground paint,
the selected endpoint switches at 0.5; a plain endpoint is promoted to
solid paint. Merging or applying a plain color does not displace an
existing foreground paint. Applying a plain color without one now also
overrides an inherited style rather than silently keeping it.

Before the repair, Incular sometimes emitted its default glyph color
instead of the configured foreground. Retained Text, rich spans,
SelectableText and EditableText now select the effective foreground.
The new tests inspect the glyph commands, verify differently painted
inline spans, and show that a foreground-only update preserves the
render object and layout count. The authored Dart probe checks actual
opaque glyph pixels at six representative merge and tween states.
Forty-five focused Rust tests passed.

The original registration remains partial because Flutter exposes
nullable simple color and a general engine Paint object. Incular's
foreground descriptor is a solid color, not the full Paint API.
This does not compare Incular GPU pixels.
