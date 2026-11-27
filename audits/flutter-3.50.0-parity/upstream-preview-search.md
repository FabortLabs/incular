# Preview search query and clearing

The exact pinned `preview_search_controls_test.dart`:12 case passed.
Incular mounts a neutral, borderless editable search field over the
same controller the preview matcher reads. Native text input updates the
full query and removes the nonmatching rendered result. The “Clear
search” action appears for nonempty text and clears both the editor
and the filter, restoring the other result. The mounted test also
checks that the search field stays focused and typing can continue
without a second click.

The extra native cases verify programmatic text/caret synchronization
and trimmed case-insensitive matching against enabled group, name,
script, and package fields. Disabling the last searchable field is
rejected. This batch checks the search model and the editable/clear
UI; it does not claim the separate search-fields popup presentation.
Three focused and ten adjacent preview tests passed.
