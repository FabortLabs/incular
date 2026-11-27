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

## Placeholder images and painter ownership

Five pinned originals passed; four pair with the mounted native
implementation. A pending provider paints its 10 px fallback and
requests one new frame when the 100 px image arrives. A synchronous
primary never paints the fallback, even though Flutter resolves it
first. Placeholder identity participates in value equality and appears
in the diagnostic description. Unmount drops the painter's retained
primary image, verified with a weak image handle.

The primary's completion drops the fallback provider reference
immediately. A display list that already painted the fallback may
keep its own copy alive until replaced. The native regression verifies
the fallback is released after the next draw; the original checking
its exact handle count before a redraw remains partial.

All 14 focused native cases passed. Older crossfade checks now assert
the existing exact normalized 8-bit alpha (64/255 and 128/255) rather
than unrounded fractions; the blend sequence and image identities
remain exact. No GPU-image comparison is claimed.

## Provider error callback

Pinned painting/decoration_test.dart:411 now pairs. A synchronously
failed provider delivers its original typed error once, after the
painter is materialized, not from inside layout. Async errors are
reported to the live handler; an obsolete provider cannot notify a
replaced decoration. All 15 focused native tests passed.
