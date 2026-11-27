# Contact hit test reuse

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
The two original hit-count registrations in `widgets/hit_testing_test.dart`
passed on the Linux reference. Native tests count calls to a retained custom
painter's hit predicate during actual Runtime input dispatch, not calls to
a mock recognizer.

Incular previously evaluated three hits for a stationary mouse click and
four for a touch press, move and release. Chrome and text-focus checks now
reuse the press result. A same-position mouse release reuses the matching
press while the geometry is current; a frame, mutable tree borrow, pointer
boundary, device presence change, or relevant geometry revision invalidates
the snapshot. Fresh hits are still taken for hover-capable movement and for
legacy controls or selection operations that need current geometry. Ordinary
touch contact motion and release use the retained routes, including when a
host omits pressed-button bits on movement. Standalone synthetic touch hover
remains supported.

The exact hit-count cases now take one hit each. Additional native checks
cover invalidation across a frame and fresh hit testing during mouse drags.
Focused hover, cursor, list, tap-region, mobile touch focus and window chrome
regressions passed. The separate one-line result/entry description
registration remains unverified.
