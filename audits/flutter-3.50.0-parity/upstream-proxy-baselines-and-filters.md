# Transparent proxy baselines and backdrop filters

## Actual and dry baselines

Pinned `rendering/proxy_box_test.dart:1060,1074,1087` passed. They
check no dry baseline for an annotations proxy around an unaligned
child and no actual or dry baseline for a childless proxy. The
authored pinned `proxy_baselines_test.dart` also exercises the actual
semantics and backdrop widget renderers at 40×20, switching through
a box without a baseline, a paragraph with a baseline, an absent
child and a replacement box on the same render object.

Before the Incular fix neither the retained annotations wrapper nor
the backdrop filter could answer the dry size at all. Both now forward
dry constraints and baseline queries. The backdrop filter also forwards
and clears the child's actual baseline. The native fixtures assert the
dry size, use the child's actual and dry metrics for the positive
control, and check absence without synthesizing a bottom-edge baseline.
The backdrop case truly removes its child; a childless Incular
Semantics widget uses the existing zero-size box fallback.
The native baseline and adjacent backdrop targets passed all nine tests.

## Mixing direct and layout-aware backdrop filters

Pinned `rendering/proxy_box_test.dart:1033` passed. A direct zero blur
round-trips as a direct configuration. Switching through an identity
matrix, composed direct filter, and layout-aware blur replaces the
previous value. Flutter's assertion when reading a structured value
as a direct filter is an explicit `None` in the Rust API.

Incular's builder-style setters keep the mounted child and independent
backdrop options intact. The native test also records a translated
matrix followed by blur, replaces the chain with one bounded 10×10
sigma blur, and verifies the local 40×20 sample bounds. Returning
to a legacy blur restores the specialized command under the same
capture key, root render, layer and child. The focused backdrop
and shader targets passed all 22 tests. This checks retained scene
commands, not pixel equality across GPU backends.
