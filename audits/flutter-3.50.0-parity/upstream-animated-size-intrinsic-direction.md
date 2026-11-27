# AnimatedSize in intrinsic layout and directional alignment

Two pinned `widgets/animated_size_test.dart` registrations are now paired.
Under `IntrinsicHeight` and `Wrap`, the initial 100×100 layout stays unchanged
on the update frame, animates to 150×200, then tracks 111×111 and 222×222 on
consecutive unstable updates. The default clip is hard-edge. Each of the four
clip modes can be set on the same retained render node.

A supplemental Flutter comparison showed that `AlignmentDirectional.centerStart`
places a 200×200 child at (0, −25) within its 150×150 animated box in LTR.
Switching to RTL during that frame moves the child to (−50, −25) without
resetting the size or the finish time. The size API previously accepted only
physical alignment. It now resolves both kinds during builds, and the native
test passes each offset and completion boundary. All ten cases in the named
native AnimatedSize integration target passed.
