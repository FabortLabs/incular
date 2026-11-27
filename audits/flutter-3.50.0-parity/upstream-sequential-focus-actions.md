# Sequential focus action results

The pinned :3909 and :3922 registrations in `focus_traversal_test.dart`
passed. Incular's public next and previous actions report `Handled`
after a successful move and `SkipRemainingHandlers` when traversal
does not move focus. The mounted tests also invoke both actions
against the actual runtime and verify that a stop edge preserves
existing focus. The ordinary Tab/Shift+Tab fallback uses these
actions. Two focused and seven adjacent focus tests passed.
