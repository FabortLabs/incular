# Flutter toolchain build metadata

Six originals in `services/flutter_version_test.dart` pair. The
`flutter_version!()` application-side macro reads the six exact
compile-time Flutter definitions: version, channel, repository URL,
framework revision, engine revision, and Dart version. This metadata
does not affect Incular's own version or the application build number.

The native checks ran three times: with no definitions, with a mix of
explicitly empty, absent and populated definitions, and with all six
populated. An empty channel stays an empty value rather than becoming
absent, and a missing URL is not synthesized from the checkout. The
six pinned Flutter originals and all seven native tests in each of the
three builds passed.
