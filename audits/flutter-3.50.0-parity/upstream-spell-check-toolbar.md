# EditableText custom spell-check toolbar

A neutral Flutter probe exercises an actual custom suggestions toolbar
without either design system. The toolbar stays closed without focus,
results, or a caret in a misspelling. Pressing a suggestion through the
custom toolbar replaces the word, keeps focus, preserves the composing
range under an equal-length replacement, and closes the toolbar.

The focused Incular runtime tests additionally exercise read-only
transitions, retained callbacks from a prior dismissal, and a valid
span with an empty replacement list. A custom builder still runs for
that empty span; the host application supplies the presentation. A
retained button is invalid once its menu closes, a new menu opens, or
the relevant editing state changes.

The two original registrations ran in Flutter. They are partial because
they assert Material/Cupertino widgets and labels, and one also has a
web variant of its behavior which was not run in a browser. No design
system toolbar, automatic gesture policy or alternate selection paint is
claimed here.

## Alternate selection paint

The neutral Flutter fixture inspects the actual RenderEditable selection
color after rebuilding the open and then dismissed toolbar. It uses the
alternate ARGB value only while the spell-check toolbar is visible.
The Incular focused display-list test compares the same selection
rectangles before, during, and after the toolbar. Only their color
changes; the range, geometry, and original highlight are preserved.
The Incular visibility transition invalidates selection paint without
requesting editor layout on its own. This authored control does not add
an upstream registration.

## Android single-tap handles

The pinned Android test and the native runtime pointer sequence compare
three transitions: a correctly spelled word shows the caret handle;
a misspelled word opens suggestions and hides the handle; another tap on
a correctly spelled word restores the handle. The original designer toolbar
label is not part of Incular's comparison, so the registration is partial.

The neighboring Android first-drag focus expectation also failed with
this change reverted. That baseline issue is separate from spelling and
is not marked as fixed by the passing pointer sequence.

## iOS precise and touch taps

The authored Flutter fixture attaches the neutral selection gesture builder
to an EditableText. A finger tap inside a misspelling selects the word and
opens the suggestions toolbar at gesture completion. A later tap on the
same already selected word hides the open toolbar without dropping the
selection. The fixture explicitly reopens the toolbar if caret-reveal work
hides it before that second press, keeping the repeated-tap assertion
independent of the animation. A mouse tap places a collapsed caret and
does not open suggestions.

The Incular fixture drives actual pointer events under the iOS runtime
policy. Touch-down alone does not select the word before the tap wins.
This is an authored comparison, not a physical-device run or a new row
in the upstream registration count.

## Replacement caret policy

Both pinned mobile variants of `widgets/editable_text_test.dart:16888`
passed. Both intentionally supply a toolbar builder that moves the caret
to the end of the accepted word, independent of the target platform.
Incular exposes that editing behavior to a neutral toolbar without
importing a design system. Its default still adjusts the existing
selection, as the earlier neutral fixture verifies.

The mounted native editor uses a live spell service and real pointer
events at character offsets to select each misspelling. Its configured
toolbar action replaces equal-length, shorter, and longer words and
places the caret at 4, 9, and 14 respectively while keeping focus and
closing the toolbar. The new text and selection commit atomically
through the controller's user-edit path. All seven focused native
toolbar tests passed.
