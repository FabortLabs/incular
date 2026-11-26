# Secondary-click selection and focus

The two pinned Flutter text-selection registrations passed across all six
test variants. A focused Incular regression sends real right-button events
over the tenth character in a mounted editor. On the iOS/macOS runtime
policies the first right-click selects the word; on Android/Linux/Windows
it puts a collapsed caret at the click. Clicking inside an existing
selection preserves both forward and reversed base/extent on all five
Incular policies.

Before the fix, an unfocused macOS or Windows field performed the right
selection and then silently replaced it with desktop select-all as it took
focus. The click must request **internal** focus, exactly as
`EditableTextState.requestKeyboard` does, so it cannot take the
select-all-on-external-focus path.

These dispositions are partial: Incular does not have a Fuchsia host or an
EditableText selection-cause callback matching the Apple case's fake
render delegate. The focus, selection and context menu regressions ran in
the native Rust runtime with emulated platform policies, not OS dashboards.
