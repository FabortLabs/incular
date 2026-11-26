# Application build metadata

Both pinned `services/app_version_test.dart` registrations passed (lines
9 and 18). Incular embeds application metadata at the macro's call site,
not in the framework's build or from an environment read at runtime.
`INCULAR_BUILD_NAME` and `INCULAR_BUILD_NUMBER` are optional and independent.
A Rust app can supply its own manifest version as a fallback, split at
the first plus. The service preserves an explicit empty name and leading
zeros in the build number, since these are strings, not parsed integers.

The native integration target was rebuilt and passed both with neither
variable set and with name explicitly empty and number `0045`. Further
assertions exercise either override in isolation and manifest versions
with and without a number. The no-default-features façade check verified
the macro works without a native host feature. Writing that data into
Android or iOS store manifests is a separate packaging concern; no
bundle tooling or platform metadata was changed by this batch.
