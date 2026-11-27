# Preview filtering by the selected source file

The four pinned scaffold registrations at :153, :168, :186 and
:238 in `filter_by_selected_file_test.dart` passed. The Incular
filter keeps the project's source paths and visible group order
separate from the editor's current URI. POSIX paths are sensitive
to case; Windows drive letters and URI-escaped colons normalize
to the same source. Losing editor focus to a non-source window
leaves the last visible group alone.

The mounted neutral toggle is absent without Editor service
availability. When that service appears, it becomes visible and
shows no previews until a file is chosen. Disabling the filter
displays the entire group without changing the selected source.
The default does not write a preference. A user toggle persists
before changing the filtered list; a rejected write leaves both
the toolbar and the list unchanged. The test exercises real
mounted controls and rendered labels with a fake preference
transport, rather than launching an IDE or a DTD server.

Four focused and six adjacent native runtime tests passed.
