# Secondary-click selection and focus

The two pinned Flutter text-selection registrations passed across all six
test variants. A focused Incular regression sends real right-button events
over the tenth character in a mounted editor. On the iOS/macOS runtime
policies the first right-click selects the word; on Android, Fuchsia,
Linux, and Windows it puts a collapsed caret at the click. That first
selection is reported with cause Tap. Clicking inside an existing selection
preserves both forward and reversed base/extent without a selection callback
on all six policies.

Before the fix, an unfocused macOS or Windows field performed the right
selection and then silently replaced it with desktop select-all as it took
focus. The click must request **internal** focus, exactly as
`EditableTextState.requestKeyboard` does, so it cannot take the
select-all-on-external-focus path.

The focus, selection, and context menu tests run in the native Rust
runtime with explicit target platform policies, not OS dashboards.
