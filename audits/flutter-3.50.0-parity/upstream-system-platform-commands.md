# System navigator and sound channels

Pinned `services/system_navigator_test.dart:20` and `:39` pass.
The native adapter sends pop with a null animation argument on
flutter/platform. It sends both history modes and all three route
variants on flutter/navigation, with exactly the source URI,
nullable state and replacement flag. A native control preserves
explicit false rather than conflating it with an absent animation
argument.

Pinned `services/system_sound_test.dart:11` now pairs the actual
flutter/platform SystemSound.play call and click argument. The
native tests also check tick and alert names; the existing
window-bound native sound service remains unchanged. Thirteen
selected native tests passed, including continued haptic and
clipboard protocol behavior. No physical device sound, Android
activity dismissal or browser history manipulation is claimed.
