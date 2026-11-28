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
