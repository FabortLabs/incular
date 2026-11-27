# SelectableRegion sharing

Pinned `widgets/selectable_region_test.dart:5698` is paired. The native
test mounts the public SelectableRegion with a context menu builder,
touch-holds the letter “r” in “How are you?” for 500 ms, and checks
selection offsets 4 through 7. Share is the second Android button.
It emits the selected string `are` on the same modeled host route as
editable sharing, clears the region selection and highlights, and dismisses
the toolbar without stealing focus. The captured button cannot share again.
The Flutter fixture checks the real method call using a mock messenger;
the Rust context-menu suite separately verifies that the typed Share
command encodes as `Share.invoke` on `flutter/platform`.

The five supported native target policies also reproduce the exact
button-kind order from Flutter's `:5761` registration. Android yields
Copy, Share, Select All; iOS, Linux, macOS, and Windows yield Copy,
Select All. The registration is partial because Flutter also runs it
on Fuchsia, which has no separate Incular selectable-menu target.
This batch does not verify the native share sheet or handle appearance.

The Android reference case, five reference order variants, and 26
focused Rust menu, toolbar and text semantics tests passed.
