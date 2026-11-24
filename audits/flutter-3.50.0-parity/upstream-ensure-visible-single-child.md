# SingleChildScrollView ensureVisible coordinates

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

All five exact originals in `widgets/ensure_visible_test.dart` and
the focused `flutter350_ensure_visible_upstream_parity` Rust target
pass. They compare world coordinates in the original 800×600
surface, 600×400 viewport, and seven 200-pixel children.

- :72 and :99 exercise both axes, leading and trailing placement,
  first and final scroll limits, and a 1-second animated reveal
  sampled at 1020 ms. All original edge samples agree.
- :126 and :196 reverse the respective axes. The same explicit
  alignment and animation results agree. After a reset, already
  visible children do not move the viewport. KeepVisibleAtStart
  brings the offscreen target into view without over-revealing,
  and KeepVisibleAtEnd reverses the movement at the other edge.
- :271 rotates the keyed child by 180 degrees. Its transformed
  local bottom-right is the world top (100), and the local
  top-left is the world bottom (500) after trailing reveal.
  Both samples meet the original ±0.1 tolerance. AABB extrema
  are equivalent to those named corners for this exact rotation,
  not a claim that arbitrary rotation has the same mapping.

This slice does not cover ListView recycling, nested different-axis
viewports, shrinking viewports or TwoDimensionalViewport.
