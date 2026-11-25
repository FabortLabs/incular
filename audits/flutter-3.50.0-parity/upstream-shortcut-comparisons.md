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
