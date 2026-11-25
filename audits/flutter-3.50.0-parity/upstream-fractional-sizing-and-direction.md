# Fractional layout with direction and optional children

All five originals in pinned `widgets/fractionally_sized_box_test.dart`
passed. At :53, a 400×300 placeholder aligned top-end in an 800×600
RTL view is centered at (200,150); the physical top-right control from
:32 is centered at (600,150) regardless of direction. At :74, logical
top-end places the 50×25 fractional child at (25,37.5) in RTL and
(725,37.5) in LTR. Both Rust nodes move their child when direction
changes without replacing their retained render object. The :100 port
now uses a real Placeholder and runs through paint at 0×0.

The extra pinned `fractional_optional_child_test.dart` verifies bounded
childless layout at (200,225), with size 400×150, and animates an empty
box from 100×100 to 150×150 to 200×200 at 0, 100 and 200 ms. The Rust
counterparts require no dummy child; the same underlying fractional
layout now avoids a transparent leaf in a childless Container. A
logical alignment needs direction only when a child must be placed;
Incular returns a structured missing-direction error.

These tests assert geometry and absence of render errors, not raster
samples. The directional behavior is tested for the non-animated boxes;
direction changes during a running implicit alignment tween are separate.
