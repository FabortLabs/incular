# Mobile editable selection actions

The pinned `EditableText` implementation in
`packages/flutter/lib/src/widgets/editable_text.dart` builds the modern
toolbar in platform order. With a nonblank selection, Android puts Share
before Select All. iOS puts Look Up, Search Web, and Share after the basic
editing actions. Obscured, collapsed and whitespace-only selections
have none of these actions, including text consisting of a BOM and a
nonbreaking space. Read-only text can still be shared. Nonempty legacy
toolbar options bypass these defaults.

Incular previously included these descriptors only in the native system
menu. It now uses the same ordered eligibility policy for the in-widget
toolbar. Pressing an adaptive button leaves focus, selection and toolbar
in place. The host-facing command drains from Runtime, Application or
MobileAppDelegate. Its Flutter-compatible adapter sends the untrimmed
selection as a string on `flutter/platform`, with the exact
`LookUp.invoke`, `SearchWeb.invoke` or `Share.invoke` method.
A stale callback cannot send text from a replaced menu or a field that
lost focus, changed selection text or became obscured before dispatch.

The authored Flutter probe mounts a modern editable toolbar with
handle-only selection controls and checks three target-platform variants.
Clipboard and action calls use mock transports. This verifies framework
menu behavior and protocol, not native mobile picker/dictionary UI.
No credit was added for a registered upstream scenario. SelectableRegion
adaptation and host-specific Live Text remain separate.

All three reference variants and 27 focused Rust runtime regressions
passed. A MobileAppDelegate crate check was blocked by the cloud's
offline cache missing `ab_glyph_rasterizer`; its forwarding method
is not covered by these runtime tests.
