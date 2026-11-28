# Draggable target hit paths

Both pinned `widgets/draggable_test.dart:92,173` originals passed.
Their 300-wide upper target overlaps the left half of a separate
200-wide lower target. The pointer begins on the upper-only portion
and then enters the overlap. If the upper target rejected the payload,
entering the overlap now offers the lower target a chance to accept.
If the upper had accepted, it remains the only target to receive
the drop and the lower is never offered the payload.

Before the fix Incular searched only ancestors of the ordinary hit
target, so a translucent DragTarget with an empty child did not
receive an offer. The drag path now reads retained hit annotations
frontmost first, and a matching prefix can skip re-entry only after
acceptance or when the number of candidates did not grow. The mounted
native fixture compares actual offers and final accepted payloads.
Native controls also verify that a defer-to-child empty upper target
does not participate and an opaque upper target blocks a lower one.
All 35 focused mouse, target path and drag option tests passed.

## Feedback positions and update callbacks by axis

All eight pinned `widgets/draggable_test.dart` registrations at
:1108, :1125, :1142, :1162, :1179, :1248, :1285, and :1322 passed.
The native widget fixture tracks the actual feedback element and
its drag position after the original is replaced. Unrestricted
moves of ±(300,300) retain both components; locking each axis
retains only its component on both straight and diagonal moves.
Release removes feedback and restores the source.

The three update traces also compare real callback deltas. Before
movement the down-created feedback sends no update. The free drag
reports (10,10) once; vertical and horizontal locks report only
(0,10) or (10,0). Zero and suppressed-axis movement and release
do not add callbacks. Both focused native matrix tests passed.

## Detaching an entered target while dragging

Pinned :2173 and :2789 passed. The mounted native comparison first lets
a control tap finish one drag. It then moves a second drag onto an
accepting target and removes the target while the pointer is held.
Removal does not end the source or deliver a drop. The release ends
the still-active source but does not call the detached target.

Previously the removed target remained in the active drag and failed
the retained-tree invariant with a dead element ID. Unmount now
silently drops both accepted and rejected target records for that
element. The native fixture also verifies that no stale leave is
delivered and checks tree invariants between removal and release.
All 27 focused native tests passed.

Separate pinned registrations :2849 and :2514 confirm that Flutter
keeps an established avatar alive when only its source disappears and
can present two independent avatars from one source. Incular's
separate mouse-source removal check currently cancels its local
avatar, and the typed context then had a single feedback slot.
Independent feedback was repaired below; source lifetime remains open.

## Terminal pointer samples and diagonal locked-axis updates

The authored pinned cases send a horizontal-axis drag from (20,60)
through (145,75), then release or cancel at (260,80) over a different
target. Flutter reports one original pointer delta, (125,15), while
moving feedback only horizontally. It offers only the first target
and finishes from feedback position (135,50). Release drops there;
cancel does not drop. A terminal sample is not a movement update.

The native baseline instead reported (125,0) and a spurious (115,0)
on both terminal events. Incular now filters displacement but keeps
the original delta for callbacks that do fire. Release or cancel
uses the last real drag placement without a new offer or move.
Both pinned authored probes and all 40 focused native tests passed.
This authored comparison does not change the upstream count.

## Typed payload movement and leave callbacks

Pinned :15, :256, :328, :399 and :467 passed. With the pinned
FlutterTest font, the native control starts once on press, paints
feedback, sends one target move, and delivers payload 1 to both
drop callbacks exactly once at (358,57). Release removes feedback
and does not add another move.

Moving across two separate targets and back emits exactly three move
callbacks in order (first, second, first) and two leave callbacks in
order (first, second). Accepted release adds neither a move nor a
leave. The Dart callback signatures vary between precise and widened
types; native uses their equivalent typed closure and compares the
same delivered values and behavior. Both focused native tests ran
with the extracted embedded FlutterTest font.

## Independent active contacts and the drag limit

Pinned :2514 now pairs. A zero limit starts no feedback; a limit of
two admits two genuine touch contacts, rejects the third, and retains
both independently placed feedback widgets. Releasing one drops
payload 1 at (358,57) and removes only its own widget. Releasing the
second produces the same drop once; the refused third emits nothing.
Before the fix one contact replaced the other's feedback and could
clear its payload on release.

Each live drag now owns its position, grab offset, feedback, payload
and target-candidate identity. The same-context authored comparison
uses two separate sources and payloads 7 and 9. They track different
positions and both enter the same target. Canceling 7 leaves only 9
as a live candidate and does not remove 9's feedback or drop. This
does not change the context's single-snapshot convenience API; its
automatic overlay presents every active contact. The 44-case focused
regression run and both multi-contact checks passed.

### Rechecking capacity when a pending gesture wins

Two touch contacts first compete with separate child tap recognizers
under one source with a limit of one. Both can be enrolled before
either starts, but only the first accepted drag may create feedback.
The other contact remains refused even after the first releases.
Pinned Flutter passed; native previously started both. Incular now
checks capacity at activation as well as press and remembers refusal
for the rest of that contact. The authored fixture and 32 focused
native tests passed without adding an upstream registration.

## Feedback hit testing and accessibility

Pinned :3906, :3938 and :3970 passed. The native fixtures use the
active pointer route: mouse feedback under a stationary pointer
selects Grabbing only when its pointer exclusion is disabled. A
second touch can tap that opted-in feedback during either an immediate
or a long-press drag, without ending the first contact. The default
remains pointer-transparent.

Feedback accessibility now defaults to excluded and can be enabled
independently of pointer hits. The two authored Flutter cases and the
native semantics tree agree on label presence before feedback teardown.
All seven focused native cases passed. The comparison checks
framework cursor selection and semantics, not OS presentation.

## Source and target hit policies

Pinned :3395, :3886 and :4006 passed. The first registration's title
says DragTarget but it actually configures an opaque LongPressDraggable.
The native getters distinguish the ordinary source's defer-to-child
default, the long-press opaque override, and the target's translucent
default versus its explicit defer-to-child setting.

Mounted controls verify what those options do. An empty default
source leaves a lower tap target alone; an opaque source starts from
its empty bounds and blocks that target. A translucent source leaves
a tap available to the lower sibling but competes and can win when
the pointer actually drags. Opaque long press starts at the deadline
over the empty child. The authored Flutter cases and all 21 focused
native tests passed.

## Button-mask admission

Pinned :4045 and the mounted native fixture agree on default
primary-only admission, an explicit secondary-only filter rejecting
primary, a primary-or-tertiary filter accepting primary, and an
always-false filter. A pinned authored mouse fixture also checks
actual secondary and tertiary drags, an accepted chord, and rejection
of a default primary-plus-secondary chord. The native callback sees
the entire normalized mask once per press. This changes only the
Draggable member's admission, not sibling tap policies. All 19
focused native tests passed.
