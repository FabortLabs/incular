# Asynchronous decoration images

Pinned Flutter 3.50.0 `widgets/box_decoration_test.dart` lines 55
and 77 passed. The native runtime uses a deliberately unfinished
provider and real retained frames. It stays idle until completion,
schedules a frame for the completed result, paints the decoded
image, and then returns to idle. The keyed reparent test preserves
the mounted subtree and pending resolution and draws the image at
its new world position, not the old one.

Additional native controls ensure bytes-received progress doesn't
repaint a decoration that has no progress UI, a followed stream
delivers its eventual result, and finishing an obsolete provider
does not wake or repaint its replacement. Ordinary image widgets'
progress behavior and the existing decoded decoration path remain
covered. All 29 focused native checks passed.

`DecorationImage::new(decoded_handle)` and its builder remain
valid. The public image field now distinguishes decoded handles
from providers; code that inspects the previously decoded handle
can use `decoration.image.decoded()`. Providers use the owning
view's image configuration and the measured decoration size.
