# Emergency line breaking for bounded paragraphs

A paragraph with 100 unbroken letters was measured under the exact
30px-width constraint in Flutter's rendering flow-layout case. A pinned
authored TextPainter probe also checks that the same paragraph wraps
into multiple lines at 30px but remains one line at its natural width.
Incular previously kept the whole word on one line when bounded.

The retained text engine now allows a shaped-cluster fallback when a
word is too long for a line. Unbounded and explicitly nonwrapping
layout keep one line, and normal break opportunities and intrinsic
word widths remain intact. The Rust engine and grapheme-selection
tests ran against the new shaping policy. This is an authored
text-engine comparison; the constraints render registration is tracked
separately.
