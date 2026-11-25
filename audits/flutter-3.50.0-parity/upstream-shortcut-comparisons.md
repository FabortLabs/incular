# Shortcut character and repeat checks

Pinned Flutter 3.50.0 `ab2912035e`, Incular `c647cb85`.
Flutter's five selected original registrations passed (eight runtime
expansions). Two are paired end-to-end through normalized public
keydown/repeat/keyup: `shortcuts_test.dart:1425,1446`. Both Incular
controls match: down/repeat/up invokes the callback counts [1,2,2]
by default and [1,1,1] when repeats are disabled. The Rust target
`flutter_character_shortcut_dispatch_parity` does not emulate Flutter's
RawKeyEvent vs KeyData simulator transports.

The three original single-activator registrations remain reference-only
as whole tests: Ctrl-C transition order and NumLock state tracking
were not reproduced end-to-end. Direct predicate controls verify a
bounded subset: repeat options, exact Ctrl/Shift/Alt/Meta and ignoring
NumLock for an already-normalized key. Lock modes are not implemented
as a configurable public activator option in Incular.

The paired case-sensitive character regression is separate:
`CharacterActivator('a')` and `CharacterActivator('A')` match only
their produced case in Flutter. Incular accepts both. The Rust target
`flutter_shortcut_activator_parity` fails at that assertion; run its
pinned Flutter counterpart with
`./audits/flutter-3.50.0-parity/run-reference-probe.ps1 -Test shortcut_character_case_reference_test.dart`.
This is a semantic difference, not a claim about OS key translation.

## Num Lock and mounted callback scopes

Five exact originals at :528, :571, :614, :1602 and :1637 passed.
Rust verifies both Num Lock states for locked, unlocked and ignored
activators after real Runtime key dispatch. The state reaches mounted
callback shortcuts separately from native modifier flags. Exact callback
controls cover distinct keys, no dispatch on release and inner-scope
propagation. One authored reference also confirms that multiple distinct
matches run in order and a duplicate binding replaces in place.
Thirteen focused Rust checks, five Flutter originals and the authored
probe passed. This does not verify all native lock key normalization.

## Produced character sensitivity

The previously reproduced case-folding difference is fixed:
`CharacterActivator` compares the produced character verbatim while
still ignoring Shift as an independent modifier. The pinned authored
lowercase/uppercase probe and four exact Flutter test invocations passed
for the three registrations :485, :1406 and :1516. Rust matches produced
question mark through the actual mounted registry and verifies both
repeat policies and ignored releases. Sixteen focused Rust tests passed.
Native character normalization itself is unchanged.

## Logical Ctrl-C chords

Four exact upstream registrations at :283, :382, :408 and :435 passed
(seven invocations including the raw-key and key-data variants). The
four Rust counterparts mount an autofocus target and invoke the public
CallbackShortcuts path. They check eight Ctrl-C sequences, either
Control position or both at once, unrelated held keys, late modifiers,
Shift/Ctrl order, repeat defaults and opt-out, and the final empty
pressed-key state. These are normalized logical-key tests. They do not
assert that a native adapter separates a layout's logical key from the
character actually produced by typing it.

## Registered shortcut routing and Num Lock

Three exact originals at :1774, :1884 and :1921 passed with mounted
registrars. Each binding fires on its own key-down, a matching inner
registrar prevents a matching outer registrar from running, and an
unmatched key can reach the outer registrar while the inner key is held.
Neither scope invokes callbacks on key-up. Rust uses owned callback
registrations instead of Flutter's VoidCallbackIntent dispatch.

A separate authored Flutter probe, `shortcut_registrar_num_lock_test.dart`,
and a focused Rust regression exercise Num Lock off/on/off without
putting the tracked mode into the event modifiers. This originally
failed in the Rust registrar because it ignored the hardware snapshot.
The registrar now takes the same snapshot as CallbackShortcuts while
leaving the registry's reported native modifiers unchanged. Fourteen
focused Rust tests passed after the fix.

## Mounted character callbacks

`CallbackShortcuts` now accepts the same `CharacterActivator` used by
registrars, while the existing widgets import path remains available.
Three originals at :1467, :1667 and :1703 passed (four exact Flutter
invocations). Incular matches the two modifier sides, requires Alt,
Ctrl and Meta together, and does not fire when only Shift is held.
Non-overlapping nested callbacks both work, and nested callbacks and
typed command shortcuts are resolved in focus-ancestor order. The
command test uses Incular's logical bindings for Flutter's single-key
LogicalKeySets. Thirteen focused Rust tests passed, including prior
registry case and callback Num Lock controls.
