# Physical accessibility actions for drag gestures

Pinned `rendering/proxy_box_test.dart:100` starts with a tap
and horizontal drag handler. Flutter advertises tap, ScrollLeft
and ScrollRight; narrowing the allowlist to tap and ScrollLeft
removes only ScrollRight. Before the repair Incular advertised
only tap. The retained Rust regression checks the exact action
lists and stable semantic identity across the allowlist update.

An authored Flutter probe and an AccessKit-to-Runtime test
exercise all four directions on a 200×100 detector translated
by (20,30). The default 0.8 factor produces deltas (-160,0),
(160,0), (0,-80) and (0,80), and the transformed center is
(120,80). As in Flutter, primary delta matches the signed
axis; no source timestamp is invented, and localPosition
defaults to globalPosition for these synthetic events. Flutter
uses a null pointer device; Incular reports its Unknown kind.

Changing the factor to 0.5 and retaining only tap, left and
down yields -100 and 50 on the allowed axes. A stale native
request for right is rejected by the live descriptor. Separate
Rust controls confirm that drag-only gestures are accessible,
pointer blockers remove the arrows, and excluding gesture
semantics does not disable a physical tap.

Native projection now maps explicit directions separately.
Existing logical forward/backward actions on scrollable views
remain supported and their legacy projection tests pass.
These probes use the actual portable graph and native data
adapter, not a live OS screen reader.
