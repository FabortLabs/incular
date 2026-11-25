# Focus widget semantics

## Pinning the initial behavior

Five exact-name Flutter `widgets/focus_scope_test.dart` registrations
passed on the pinned reference. The Runtime target
`flutter350_focus_widget_semantics` exposes two current failures:

- :1358 Keyboard focus itself is gained and retired correctly, but the
  retained label never reports that it is focusable or focused. It should
  start focusable/unfocused, become focused, and withdraw the flags when
  the same node is no longer eligible.
- :1846 A single RTL text label preserves its semantic identity and
  direction. However, it has no Focus action and dispatching a focus
  request against its semantic node does not move keyboard focus to the
  surrounding Focus widget. Flutter does both. The Rust test checks
  the actual action dispatch, retained node and input focus, not just API names.

The test for :1886 verifies that the mounted Focus widget measures to
zero under a zero-area view and can then process a focus request without
a panic. It passes.

:1633 and :1085 are reference-only for now. Incular's Focus has no
includeSemantics builder, and its FocusScope cannot be supplied the
external FocusScopeNode the original uses. An absent annotation by default
or a different no-crash widget would not cover those behaviors.

## Authored annotation-boundary controls

The pinned Linux/iOS reference probe
`focus_semantics_annotation_reference_test.dart` establishes the shape
of the fix. Two ordinary descendant labels are joined with a newline in
one Focus node. A nested Focus retains its own label and focus action;
an explicitly separate semantic control also stays independent. In
particular iOS still exposes the focusable and focused state, but its
Focus widget omits the input-focus action that Linux exposes.
These authored cases do not add original registration credit.

## Retained focus annotation parity

:1358 and :1846 now pass on the public Runtime. Focus exposes the state
of its actual input focus node and can route the advertised action to
that node, while preserving the semantic node's identity and direction.
Disabling eligibility withdraws the action and state; restoring it
without giving the node focus invalidates the retained annotation as
well. :1633 is now fully paired: the explicit opt-out produces no
semantic node for an empty child, and keyboard requests still work.

The authored Linux/iOS controls pass in Incular too. Plain descendant
labels merge with a newline, live label changes update the same ID,
nested Focus nodes and independent controls keep their own actions,
and iOS reports focus state without advertising an input-focus action.
The focus observer only invalidates annotated subtrees when eligibility
or primary focus changes. Nine targeted Runtime and Widgets suites passed
(81 tests). This does not claim native screen reader integration or the
external FocusScopeNode API (:1085).

## Focus key routing and handler replacement

Pinned originals :1643 and :1696 passed. The latter is a partial
comparison: rebuilding Incular's mounted Focus replaces the typed
handler without losing focus, and the next Enter event reaches the
new handler and the correct retained node. It does not reproduce
Flutter's public callback-property getters on FocusNode or the
deprecated RawKeyEvent handler. :1643 is therefore reference-only.

The separate pinned `focus_key_routing_reference_test.dart` checks all
three outcomes: ignored bubbles; handled stops and reports consumption;
skip-remaining stops without reporting consumption. Both the reference
and Incular route to the same inner/outer sequence after updates.
Incular also verifies on actual Tab input that skip-remaining prevents
the default framework traversal and still returns unconsumed at the
Runtime boundary. Tests use normalized keyboard events, not native OS
delivery or system shortcuts. Five focused Rust targets passed (20 tests).

## Retained identity across focus reparenting

The pinned `widgets/focus_manager_test.dart:1825` original and the
mounted Incular fixture both keep the child focused after a GlobalKey
move and update the new ancestor's focus. Incular checks both a pending
request and already-applied focus, the same retained element ID, and loss
of focus on the old parent. This is partial: Flutter explicitly supplies
separate FocusScopeNodes and requests scope autofocus; the Rust fixture
uses a queued child request and mounted Focus nodes inside the two policy
scopes. The pinned :1752 and the focused Rust replacement case were also
rerun; detached autofocus replacement is now a match in the ledger.
Six focused Rust tests passed.
