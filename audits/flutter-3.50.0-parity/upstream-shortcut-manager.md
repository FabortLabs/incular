# Live shortcut manager and focus routing

Reference: Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`.

A native `ShortcutScope::manager` reads the supplied `ShortcutManager`
mapping when a key reaches the mounted boundary. Its default map is
empty. A manager sees the actual Left Shift trigger when a descendant
has primary focus, while an unfocused boundary does not inspect or run
the key. Replacing the map updates the next key without moving or
remounting the focused target. A modal miss stops the framework focus
chain but does not consume the host key.

All five pinned originals (723, 736, 750, 786, 822), two authored Flutter
cases (live replacement and a modal miss), and 17 focused native tests
passed. The adjacent native checks cover editor input, custom activators,
and passive shortcut boundaries.
