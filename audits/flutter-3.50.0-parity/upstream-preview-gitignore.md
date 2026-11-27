# Ignore generated widget preview scaffolds

All three Flutter 3.50.0 registrations in
`packages/flutter_tools/test/general.shard/migrations/widget_preview_gitignore_migration_test.dart`
passed: absent file, already-migrated file, and append to a previously
unmigrated file.

The equivalent native tests use actual temporary project directories.
No new .gitignore is created for a project without one, an existing matching
rule keeps the exact file bytes and timestamp, and the migration appends
the expected comment and scaffold rule after the user's content. The native
helper takes a project-relative scaffold path because Incular manifests
let the host choose where the scaffold lives.

The other three native cases check CRLF and an intervening negation,
reject a path outside the project, and avoid writing through a symbolic
link. Existing regular-file permissions and bytes are preserved when
the atomic replacement appends text. Incular checks literal active rules
rather than treating a mention in a comment as an applicable ignore rule.

All six native cases passed. This is the explicit migration operation,
not an implicit mutation when reading preview metadata and not a full
preview launcher integration.
