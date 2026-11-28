# Repaint boundary capture

Pinned rendering `proxy_box_test.dart:167,258` pass. They exercise
logical 100 × 200 with image ratios 1 and 2, a partly transparent black
child with an opaque white sibling, and the same layer captured with a
negative origin at both ratios.

Incular now records a painted RepaintBoundary or translated layer for
a separate image target. The retained tests verify local origin and
crop placement, independent and ceil-rounded image dimensions, removal of
surrounding window content, retention of child opacity and transforms, and
rejection until the boundary has current paint. Recording does not change
the published window scene, its diagnostics, or composition callbacks.

GPU rasterization/readback and the image pixel assertions remain open.
Neither pinned registration is credited from command geometry alone.
