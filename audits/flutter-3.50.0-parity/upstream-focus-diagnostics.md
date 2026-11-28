# Focus diagnostics

## Node and scope properties

The pinned Flutter originals at `widgets/focus_manager_test.dart:246` and
`:1641` passed. Detached native nodes and scopes produce the same six lines,
in order, including the null context and the default flags. Those defaults
remain inspectable but do not crowd a normal deep tree at debug level. A
second test checks the live flags, identity labels, focus role, and the
newest-first history after a child changes scopes or is dropped. The seven
targeted diagnostics and scope transition tests passed.
