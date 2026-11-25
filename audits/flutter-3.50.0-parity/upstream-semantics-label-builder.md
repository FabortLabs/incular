# Semantics label composition

Nineteen exact registrations in the pinned `semantics/semantics_test.dart`
(:1122 through :1345) and their paired Rust assertions cover default, empty,
and custom separators, empty parts, retaining and clearing snapshots,
Arabic/Hebrew/Latin direction embeddings in both directions, long
labels, 100 parts, and emoji/math/currency. The public API uses the same
TextDirection type as the layout API, without adding a crate dependency.
It stores the composed label in one incrementally built buffer.

The authored opposing-direction-first-part fixture confirms that Flutter
leaves the first part unchanged, including a single mismatched part, and
adds embedding marks only to later opposing parts. It is not an extra
upstream registration. These are label composition checks, not native
screen reader or automatic merged-tree phrasing comparisons.
