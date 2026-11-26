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

### Mounted callback identity and descriptor ownership

Pinned :1696 and the externally owned Focus variant at :1750 reran.
A regular mounted Focus now installs the identical typed callback on
its node. The old callback ignores Enter; the updated callback handles
it on the same focused element. Rebuilding without a callback clears
the slot without dropping focus. An externally owned node keeps its
own typed callback across widget rebuilds. The legacy RawKeyEvent
assertions remain outside these typed checks.

The authored Flutter and native retained probes also build and discard
a descriptor that would disallow focus on the active node. Building it
no longer changes the mounted node or clears its Enter callback;
actually mounting it changes the node permissions and drops focus.
The previous native widget conversion applied permissions eagerly.
Ten focused Rust tests passed, including the existing focus and window
lifecycle controls.

### Typed and deprecated raw callbacks on the same node

Pinned :1643, :1696 and :1749 now pair at the portable keyboard
boundary. Both accessors report the identical callback on the Focus
widget and its node, inactive slots remain absent, and the retained
element keeps focus while an ignored Enter handler is replaced by a
handling one. The external node owns both live callback slots and all
four tested permissions before and after rebuild.

An authored reference and native control also invokes both callbacks
on the same key: typed first, raw second, before bubbling. Handled
dominates SkipRemainingHandlers; otherwise skip leaves the key
unconsumed while preventing ancestor handling. Native semantic-generated
keys have no raw counterpart and do not invoke the raw callback.
The portable raw view carries real logical and physical key and
state; platform-specific scan tables or native key objects are not
manufactured or included. Twelve focused Rust tests passed.

### External scope callbacks and mount-only permissions

Pinned :997 now pairs. The external FocusScope publishes both exact
callback identities from its node, not widget defaults. Enter first
falls through, then both updated callbacks handle it without moving
primary focus from the scope. Descendant focus/traversal permissions
update without disturbing the other node properties.

An authored Flutter and native control extends the discarded-descriptor
case to an externally supplied scope with a regular widget permission
override: only mounting the replacement can disable or unfocus the
live scope. Seventeen focused Rust tests passed.

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

## Initial and input-driven focus highlights

All 20 focused expansions of the six pinned highlight registrations in
`focus_manager_test.dart` passed. Incular now compares :1480, :1494,
:1507, :1520 and :1562 at the runtime input boundary: all six initial
target platform policies, first mouse movement on each, keyboard
down/up changing to traditional mode, and the ordered notifications
for key, touch, mouse, and forced strategy changes. Mouse movement
alters the initial guess on a mobile target but does not undo a real
touch interaction later. Previously the first Rust comparison observed
only the two forced strategy notifications because key and pointer
events did not reach the highlight manager.

:1528 remains reference-only. Its Android OS event includes a raw
virtual keyboard ID and flags that the normalized keyboard boundary
does not expose. Native Android IME behavior is not inferred from the
normalized hardware key test. Four focused Rust targets passed (24
tests), followed by a rerun with both key phases and real platform
startup policy in the new focus highlight target.

## Focus movement and reactive labels

The pinned originals :16, :30, and :65 in `focus_scope_test.dart`
passed by exact name. The new retained application test builds its
status text reactively from live FocusNodes. It verifies the initial
state, each node's focus state, and the exact labels in the committed
semantic tree after a request or autofocus. All three are paired. For :16 and :30
the native fixture now finds the nearest mounted scope from the keyed
control and requests focus through that scope, rather than directly
through the node.

After the primary-focus ownership fix, the pinned :812 cross-scope
case and the :1825 manager case were rerun. The native GlobalKey test
still keeps the same element, a pending or confirmed primary focus,
and the correct new ancestor focus. :812 is partial because the native
fixture does not call Flutter's root `setFirstFocus` on an explicitly
attached ScopeNode or assert the two labels during the scope move.

## Idle focus requests and ExcludeFocus

A mounted node previously queued its focus request but did not request a
frame. Native focused coverage verifies that a keyboard-only Focus with
semantics disabled schedules an idle runtime on both request and
unfocus. The exact pinned authored probe `idle_focus_request_test.dart`
settles the focus change with the framework's microtask queue without
needing another input. The Rust runtime supplies its standard frame
scheduler at attachment and removes that connection at last detach.

The pinned :1907 and :2007 `ExcludeFocus` originals passed by exact
name. Native tests verify both an outer and an inner request are
denied without a focus-change callback, and that the wrapper adds
no accessibility node versus the same child alone.

## A focused sibling survives a neighboring rebuild

The pinned authored Flutter comparison and the native pointer/focus
target pass. Focus owns the semantic node and takes its label from its
child by default; a stale native fixture was looking for a separate
Text node. The corrected check still requires both semantic IDs and
element identities to survive, the left node to preserve primary
focus and its label, and the right node to publish only its new label.

## Virtual Android keys keep touch highlights

Pinned `widgets/focus_manager_test.dart:1528` passes on the Android
test variant. The portable runtime previously treated every raw key
as physical input. Android now forwards the raw soft-keyboard flag and
device ID; either indicator classifies the event as virtual. Virtual
keys still follow ordinary routing and edit the active field, but
they do not switch automatic focus highlights out of touch mode.

The native fixture checks the exact virtual Backspace with both
indicators, each indicator alone, and the positive hardware-key
control. Nine focused highlight and key-routing tests passed. The
crate compiled on Linux and the Kotlin/JNI signatures were checked
against each other; there was no Android SDK or Android device run.
The pairing is at the normalized runtime boundary.
