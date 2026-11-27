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

This change exposes a platform-service controller, not an iOS menu host or
an automatic `EditableText` hookup. The widget-host test :459 remains
uncredited. Native errors use typed Rust results rather than Dart asserts.

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
