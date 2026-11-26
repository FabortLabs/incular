# TableBorder construction, interpolation, and paint

Pinned Flutter 3.50.0, `rendering/table_border_test.dart`.

- :9/:33/:46: default missing sides, uniform and symmetric constructors,
  physical dimensions, uniformity, and scaling through 0/0.5/2 match.
- :67: each side interpolates independently, including extrapolation
  beyond both endpoints and a non-round interior fraction.
- :120: absent endpoints scale the other border; two absences stay absent.

The portable painter records no commands for the default border. Its
standalone test checks exact divider offsets and widths, vertical before
horizontal, and the outer edge last. It also checks that different outer
widths with the same visible color emit a rounded ring. These are command
checks, not comparisons of anti-aliasing pixels. The widget wiring is
covered separately.

Run `cargo test --locked -p incular-widgets --test flutter350_table_border`.

## Retained widget border

The pinned Table smoke case at `widgets/table_test.dart:252` passes on
both frameworks. The authored control verifies no default stroke, draws
the requested inside and outside strokes after cell paint, and removes
them after a retained update to an empty border. The independent Rust
test checks that the vertical stroke follows the 60/40 column boundary
from x=60 LTR to x=40 RTL. Divider height follows the 30-pixel row extent,
not the 80-pixel outer box. It checks the command geometry, not raster
pixels or empty-table stroke degeneracies.

## Rounded outer rings and independent dividers

Pinned `rendering/table_border_test.dart:145,150,177` passed by exact name.
The native tests use the same default width, colors, 8-pixel radius, and
uneven side widths. The painted path leaves the corner open and measures
each stroke independently. Separately colored dividers make the whole
border nonuniform without changing the uniform outer ring; paint still
draws the vertical line, horizontal line, and rounded ring in that order.
The test samples the actual retained path on both sides of each intended
edge so an unnoticed extra pixel would fail. All eight focused table-border
tests passed.
