# Startup keyboard state

Pinned Flutter 3.50.0 `services/binding_test.dart:185` passed. At
startup the opt-in Incular adapter sends the standard binary
`getKeyboardState` call to `flutter/keyboard` with null arguments.
The exact `{1: 1}` reply yields one physical and one logical key,
both with ID 1, without discarding the unrecognized catalog value.

The native control also checks a merged re-sync, a 64-bit logical
ID, missing replies, an invalid map that makes no partial update,
and a later event clearing the original press by physical key.
Two focused comparisons and seven standard-codec neighbors passed.
This does not replace the default native keyboard event stream or
claim parity of Flutter's raw/key-data arbitration.
