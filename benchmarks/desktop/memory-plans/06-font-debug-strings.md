# Share font diagnostic strings

Status: proposed. Small independent candidate after 00; coordinate text API with 05/07.

A related [font-run duplication fix](06-font-run-duplication-results.md) is
implemented and measured separately. Sharing the public diagnostic strings
themselves remains proposed.

## Finding and scope

`FontRunDebug` in `crates/incular-text/src/engine.rs` owns family and script Strings.
Construction currently repeats constant descriptions, and paragraph assembly clones
diagnostic runs. Audit runtime/DevTools consumers as well as public text consumers.

## Implementation sequence

1. Count runs, distinct values, capacities and clone allocations in label-heavy,
   multiline and multilingual workloads.
2. Compare borrowed static data via Cow, shared Arc strings and compact internal
   IDs. Preserve dynamic descriptions as a supported case. Public field type
   changes need a compatibility decision before implementation.
3. Share fixed descriptions without a global unbounded interner. If real font-name
   interning pays off, keep bounded ownership at the text/font owner and define
   invalidation on font database changes.
4. Consider on-demand public diagnostic conversion only if it avoids permanently
   retaining a second representation. Keep diagnostics available when requested.
5. Ensure paragraph range rebasing does not deep-copy immutable descriptions.
   Keep this patch separate from shaping and glyph-storage changes.

## Evidence, validation and acceptance

Measure string payload, per-run metadata, reference-count overhead and allocation
count. Shared ownership is not automatically smaller than a borrowed constant.
Expect a modest saving, not an assumed solution to the full memory gap.

Use focused engine cases for mixed fonts, multiple paragraphs, registered-font
invalidation and unchanged diagnostic ranges/content. Add retention/clone coverage
through supported APIs under tests where needed.

Accept fewer retained allocations without lost information or unplanned public
breakage. Deliver occupancy numbers, representation rationale, isolated patch and
protocol-00 measurements. Revert if representation overhead outweighs the saving.
