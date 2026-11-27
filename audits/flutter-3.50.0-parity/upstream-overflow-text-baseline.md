# Overflow box preserves its child's text baseline

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Reference: `packages/flutter/test/rendering/overflow_test.dart:13`.

The native target lays out and paints a normal paragraph and one under a
top-left OverflowBox with a maximum height of half the normal paragraph
height. Both paths measure retained baselines, not a font-specific constant.
The constrained child is half-height and still reports the original
alphabetic baseline, including when that baseline lies below its layout
box. The pinned original and the focused native overflow and adjacent
FittedBox baseline tests passed. No layout implementation change was needed.
