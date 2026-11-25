# Flutter 3.50 display features and sub-screen layout

Pinned `widgets/display_feature_sub_screen_test.dart:12,36,65,94,122,
155,196,227` passed. Rust repeats the missing-Directionality diagnostic,
explicit and infinite anchors, horizontal and crossed hinges,
non-spanning notches and a flat fold, a zero-width half-opened fold,
and zero-sized layout. The box coordinates and sizes are exact against
the 800×600 reference fixture. An explicit anchor needs no direction.
A notch inside a panel no longer incorrectly narrows that panel.

Pinned `widgets/media_query_test.dart:1518,1598` also passed. Tests compare
the three adjusted inset families, preserved viewport/scale/accessibility
values, excluded hinge, and surviving cutout. A covered persistent
left inset of 46 becomes 4 at x=42, rather than zero. The sub-screen
widget now installs this real adjusted MediaQuery, taking its geometry
from the nearest scope rather than always from the native window.
