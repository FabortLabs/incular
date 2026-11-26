# Mounted focus geometry follows transformed corners

Pinned `widgets/focus_manager_test.dart:75` passes. The native scene
uses the same 8px inset, 10/20 translation, 0.33 centered scale and
180° centered rotation. It matches the unrotated node at (300,8)
with size (200,100) and the rotated node at (443,194.5) with
signed size (-66,-33), subject to f32 representation. Both
nodes report those values before any traversal pass.

Previously, the public focus rect could remain empty until
traversal and then use sorted paint bounds. The rect now records
the transformed corners in original order, including negative
extents. `FocusNode::bounds()` supplies the normalized paint
rectangle where an Incular caller previously used `rect()`
for that purpose. The Rust type keeps signed focus extents
separate from the non-negative layout `Size`. Reading and
directional traversal continue using normalized geometry.

Standalone and runtime-hosted trees both publish the geometry.
The second retained test uses a node excluded from traversal
and changes only its paint transform. Its geometry updates without
requiring it to acquire focus. Refresh work visits attached nodes
when retained layout/paint geometry changes, not every tree element
on an ordinary frame. Thirty-six focused native cases passed.

```sh
cargo test --locked -j 1 -p incular-widgets --test flutter350_focus_geometry --test focus_directional --test focus_keyboard --test pointer_event_fields -p incular-gestures --test focus_reading_order --test flutter350_pointer_event_primitives -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh upstream widgets/focus_manager_test.dart 'FocusNode Geometry is transformed properly.'
```
