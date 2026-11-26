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
