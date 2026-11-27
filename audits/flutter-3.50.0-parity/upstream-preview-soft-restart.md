# Soft restart of a single preview

The exact pinned `soft_restart_test.dart`:18 case passed. The
mounted neutral “Hot restart” button requests a local restart. The
next painted frame replaces the preview with a placeholder at its
last measured size. A post-frame callback requests a rebuild and the
following frame mounts a fresh subtree. The focused test observes
the true/false lifecycle, sees the text leave and return, and
verifies that the element identity changes and construction runs
again. Another mounted preview keeps its identity throughout.

Repeated requests before the placeholder frame coalesce. This is
not a restart of the preview process or an IDE connection. One
focused and five adjacent native preview tests passed.
