# FadeTransition build, paint, and zero-area behavior

All three pinned `widgets/fade_transition_test.dart` registrations
(:10, :35, :58) passed. The retained native tree matches the two-second
forward fade without re-entering its child's builder after mounting.
Samples at 0, 500, 1000 and 2000 ms verify the recorded opacity bytes
0, 64, 128 and 255 while preserving child and render identity.

The repaint regression applies the original 1→0.5, explicit leaf
repaint, 0 sequence before requesting a frame. It finishes without
an error and records no hidden pixels. Reappearing at opacity 1 also
proves that the pending child repaint was not dropped. A visible
fade nested under Directionality and Center finishes both layout and
paint with a measured zero-area box. All six focused native tests
passed. There is no requirement to match Flutter's build debug log
strings or a native-surface screenshot.
