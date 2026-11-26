# Editable IME candidate anchor

Pinned `widgets/editable_text_test.dart:6593,6635,6653,6673` passed.
The native comparison reads the real runtime text-input command
queue. With Ahem at 14 pixels, the caret at the end of 50 characters
is (700, 0) and 2 × 14 pixels, and moving to the beginning sends (0, 0).
An unchanged frame does not resend the rectangle.

Selections 10→30 and 30→10 anchor the same client at x=140. The
directional selection endpoints still reach the client unchanged,
and the visual endpoint of the forward selection stays at x=420.
Moving the logical selection start to 12 moves the host anchor to
x=168 without reconnecting. On the unchanged baseline the forward
selection placed the host at x=420, and the collapsed caret was sent
as a one-pixel rectangle rather than its actual two-pixel width.

The runtime now uses the normalized selection start for the IME,
separate from the visual endpoint used by caret reveal and menus.
Paint and the host use the same cursor dimensions. This comparison
ends at the command queue; it does not assert OS candidate-window
placement or the separate marked-text channel.

All 53 focused native runtime, caret paint, and editor geometry cases passed.
