# Flutter 3.50 asset bundle diagnostics

The original `services/asset_bundle_test.dart:254` passed on pinned
Flutter. Display prints the network bundle's five-digit hexadecimal
identity and base URL without fetching anything. The Rust test checks
that a cloned handle keeps its identity and a separate bundle gets a
different one. Existing structured Rust Debug output is unchanged.

The original `services/asset_bundle_test.dart:261` passed on pinned
Flutter. A missing key in the Rust root bundle retains its typed key
and reports the same two-line diagnostic, including both full stops.
The same test establishes that explicitly supplied zero-byte data is
not treated as an absent response. This matches Flutter's null-reply
check, without relying on the filesystem or transport.

All 15 focused native bundle, network bundle, and Flutter asset bundle
parity checks passed.
