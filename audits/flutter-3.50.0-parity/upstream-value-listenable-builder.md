# ValueListenableBuilder lifecycle

All seven exact `widgets/value_listenable_builder_test.dart` cases
passed on pinned Flutter 3.50.0. The retained Incular fixture checks a
null value, the initial non-null value, updates between distinct
strings, changing the source without remounting the builder, the old
source stopping immediately, unsubscribing after removal, and zero
area. Instead of observing just the new label after a change, the
old-source control checks the actual subscriber list and that a stale
notification produces no rebuild.

An authored control shows that an equal value does not trigger the
builder and that a passed-through static child retains its element ID
after a real update. Eight focused comparisons and three adjacent
listener tests passed.
