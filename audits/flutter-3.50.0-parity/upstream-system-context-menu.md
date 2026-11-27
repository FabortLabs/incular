# System text-context menu controller

## Controller ownership and platform commands

The pinned Flutter 3.50.0 `services/system_context_menu_controller_test.dart`
registrations compare against native `flutter350_system_context_menu`:
- :34 initial hidden state, exact logical target rects, duplicate show/hide
  suppression, rectangle updates, and ability to reshow after hiding.
- :128 inbound platform dismissal clears visibility, calls the system-hide
  handler exactly once, and does not issue a redundant outbound hide.
- :210, :248 one visible controller at a time, rejection of a second
  simultaneous distinct menu, and transfer of control after owner hide.
- :336 the ordered cut, copy, paste, select-all, and customized search-web
  title go on the wire. Identical items at the same target are a no-op;
  replacing the items at that target republishes the menu.
- :410 an empty item list is rejected before visibility or the wire changes.
  An additional native control checks the active text-input precondition.

Native tests decode the emitted JSON method bytes, including the channel,
method, rectangle and nil hide arguments. Built-in item IDs are unused by
the pinned iOS engine; the JSON preserves their numeric legacy field with
a placeholder rather than copying Dart object hashes. Custom actions have
separate opaque wire IDs.

The platform-service controller does not implement an iOS menu host.
Native errors use typed Rust results rather than Dart asserts.

## Action lifetime and late platform messages

The pinned :537, :590, :650, :703, :726, and :767 registrations passed.
Native tests obtain IDs from the emitted JSON, send actual platform
method bytes, and ensure only the intended action runs. Owner changes,
platform dismissal and explicit hide revoke old actions, an unknown ID
cannot invoke anything, and replacing an existing menu revokes its IDs.
IDs are opaque and not derived from Rust pointer addresses or Dart hashes.

The pinned :687 test also ran, but is partial: Dart's default messenger
absorbs the unregistered-client error and the inbound call returns
normally. This controller deliberately exposes the typed error to its
embedding dispatcher; no full messenger error-envelope behavior is
claimed. Ten native context-menu tests and four native receiver controls
passed.

## Mounted editor defaults and selection rectangle

The exact pinned :459 registration passed. The native runtime pairs its
autofocused, right-aligned multiline editor and retains a real text input
connection. The platform command uses the default Cut, Copy, Paste, and
Select All items in that order, opens the controller, and does not shift
text focus. A control rejects menu requests without a focused editor.

The authored Ahem fixture verifies replayed rectangle geometry
to 0.002 logical pixels on both frameworks, with a translated, right-aligned
400-wide editor at (13, 17). A caret sends approximately
`(390, 17, 0, 10)`, a partial one-line selection sends
`(380, 17, 20, 10)`, and a multiline selection sends `(13, 17, 400, 20)`
in either selection direction. This uses the selection, not the visible
caret's painted width. The system and existing in-widget toolbars share
the four standard button gates. Legacy controls use the declared options;
modern controls account for selection and clipboard state.

No claim is made for actual UIKit display, automatic widget lifecycle,
additional platform-only actions, complex bidi/soft-wrap endpoint
resolution, or the other `widgets/system_context_menu_test.dart` cases.

## iOS selection actions and read-only eligibility

The exact pinned widget registration :1179 focused a writable iOS-mode
editor, selected `Hello` in `Hello world`, and checked the platform's
show message for Share. Incular now sends that built-in item after
Look Up and Search Web for the same focused, retained selection.
Selecting blanks, collapsing the selection, or obscuring the field
suppresses those actions. A native localization control checks that
the nearest application catalog can change the Share title instead of
sending a hard-coded label. The three optional catalog keys are
`widgets.look_up_button_label`, `widgets.search_web_button_label`, and
`widgets.share_button_label`, with Flutter's stock English defaults.

Both pinned iOS variants at :728 passed. The mounted editor's support
query combines iOS policy, inherited media capability, and writability
without needing focus. A capable iOS view cannot opt a read-only field
into the native menu. No actual UIKit presentation is claimed.
