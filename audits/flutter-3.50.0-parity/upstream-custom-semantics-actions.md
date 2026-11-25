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

## Registration and merged dispatch

The five original registrations in `widgets/semantics_test.dart:809,835`
and `semantics/semantics_owner_test.dart:55,108,178` passed. Rust now
publishes both named actions on a plain annotation or the union on a merged
node, and dispatches by the specific canonical ID rather than by the
presence of any action. The ports verify two nested owners, a three-deep
nest, and sibling owners alongside a root owner. Each call changes only
its own callback count. An unadvertised action is not dispatched.

The authored IndexedStack case passes on both sides. Switching the selected
child keeps the merged node's identity but replaces the advertised action;
the inactive child can neither claim it nor receive a stale request.
Forty-four focused Rust checks pass across the runtime and widget semantics
targets. Point-addressed dispatch, standard action hint overrides, and
native platform presentation are not part of these five dispositions.

## Portable native bridge checks

The AccessKit projection publishes a named action's label and canonical ID.
It only accepts a nonnegative in-range ID advertised by that specific node
and rejects a stale action after the node is updated. A focused integration
test dispatches a translated request into the retained widget callback and
verifies that the live runtime also rejects it before the old projection
has caught up. The mobile projection validates per-node membership too;
Android and iOS native menu integration was not exercised.

This change does not claim live screen-reader behavior or native relabeling
of standard actions. AccessKit cannot relabel a standard action and does not
advertise an override as an additional named choice.
