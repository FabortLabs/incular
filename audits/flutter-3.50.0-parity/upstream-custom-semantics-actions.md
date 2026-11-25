# Custom semantics actions

The pinned `semantics/custom_semantics_action_test.dart:10` and matching
Rust test verify that two independently allocated equal labels use one
identifier, another label receives a different identifier, and reverse
lookup recovers the original description. A paired authored control also
verifies that overrides compare both the hint and the kind of standard
action, do not collide with a named action, and return no value for an
unknown identifier.

Incular's registry is lazy and scoped to the current UI thread (the
portable counterpart of Dart's isolate-local registry). This comparison
covers only action descriptions and canonical identity. Installing handlers,
merging them into retained nodes, and projecting the resulting actions onto
each platform's assistive technology are separate behavioral contracts.
