# DecoratedSliver paint space and order

Six pinned `widgets/decorated_sliver_test.dart` registrations pair:
four axis/reverse combinations (:186, :228, :271, :314), and the
main- and cross-axis groups (:358, :402). After a 200 px scroll the
native centered border rectangle has exactly the Flutter position
and dimensions: (0.5, −199.5) × (99, 499) vertically forward; the
vertical reversed origin is (0.5, 0.5); horizontal transposes
those coordinates and dimensions. Both groups emit a single outline
around the union, not separate borders around the constituent boxes.

The old retained wrapper applied the decoration to each materialized
child. The new retained paint layer uses the whole sliver paint extent
and cross-axis constraints, while preserving inner layout, keyed child
identity, measurement, and hit testing. A cached layer scrolls into
view without a new layout pass. An authored paired fixture changes
paint order on the mounted instance, keeps the same child, and confirms
the decoration can paint before or after the child without taking its
hit target.

Twenty focused native tests and seven Flutter comparisons passed.
A stale shared native test helper also now handles HardEdgeRect
explicitly, so the existing targeted sliver regression compiles.
These checks compare actual generated paint, not engine-specific
stroke commands or Flutter's golden image pixels. Generic external
BoxPainter lifecycle, infinite scroll, and pinned-header variants
remain outside this batch.
