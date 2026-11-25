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
