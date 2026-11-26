# StatefulBuilder and local invalidation

The pinned `widgets/basic_test.dart:1932` case passed: a real
StatefulBuilder with a placeholder in Center lays out at exactly zero
area. Incular previously offered a context Builder and reactive signals
but no StatefulBuilder with the Flutter-style local setter.

`StatefulBuilder::new` now supplies a cloneable, weak `StateSetter` for
each mount. The setter performs the mutation synchronously and
invalidates only that builder. Several updates before the next frame
coalesce. Replacing the descriptor at the same keyed element retains its
state and the handle; changing only incoming constraints does not rerun
the builder. The setter becomes invalid when its element unmounts.
Flutter throws for a stale setter in debug mode; Rust returns false
without running the mutation.

Two authored Flutter reference probes check sibling isolation, batching,
retained replacement, and disposal. Native runtime tests additionally
mount two clones of the same descriptor to verify that their identities
are independent. Native: `flutter350_stateful_builder` and
`flutter350_basic_zero_area` in `incular-runtime` (eight passed).
Only the pinned upstream registration is counted in the ledger.
