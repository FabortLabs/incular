# Grid tile and viewport extents

Seven pinned `widgets/grid_view_test.dart` registrations passed:
:505, :799, :823, :847, :875, :903 and :961. The convenience
constructors now expose the explicit main-axis extent, which both
underlying delegates honor. With 4 columns in 800 pixels, tile 4
starts at (0,100) and measures 200×100 rather than a square. A
shrink-wrapped grid under an unbounded scroll axis lays out both its
first and last tile and has a 1,000-pixel inner extent.

The two delegate variants now reject a negative explicit main extent
in debug builds, accept zero, and the maximum cross-axis constructor
rejects zero in debug builds. An authored Flutter case also pins the
positive fractional boundary: a 2-pixel grid allows four 0.5-pixel
columns, rather than silently growing the maximum extent to 1 pixel.
All fourteen focused Rust checks passed; the fractional custom probe
does not add an original registration.
