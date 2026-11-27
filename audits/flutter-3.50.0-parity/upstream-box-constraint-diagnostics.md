# Box constraint diagnostics and borrowed interpolation

Both exact pinned `rendering/box_constraints_test.dart:11,89`
registrations passed. The full descriptions show unbounded, fully
tight-infinite (“biggest”), and width-tight bounds without admitting
an infinite minimum to layout. The raw host bounds can also show
a nonnormalized interval. A Dart SDK control verifies one-decimal
midpoint rounding, and the native tests check the same labels.

Optional interpolation returns no value for two absent endpoints.
Two references to the same constraints return that actual borrowed
instance, while equal but distinct endpoints produce an owned
value. Eight focused native config tests passed.
