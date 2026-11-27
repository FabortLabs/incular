# Border and box-decoration values

Eight pinned originals and 30 focused native tests passed. These tests
cover value operations, not cross-renderer golden pixels.

BorderSide shows its bare type name when every property is default.
Non-default color, width, alignment and style remain visible. Border
descriptions collapse four equal sides and omit absent sides in a
nonuniform border. The original border interpolation controls also
pass: absent borders scale the present side, and a transition through
an invisible side changes opacity without substituting an opaque line.

The generic decoration interpolation preserves colors and returns the
same borrowed input when it is identical. A rectangular BoxDecoration
switches to the circular kind exactly at the midpoint. A missing
gradient scales the stored alpha values without destroying the
transparent endpoint's RGB; the native assertions match the original
gradient at negative, fractional, and extrapolated values using the
renderer-neutral RGBA8 representation.
