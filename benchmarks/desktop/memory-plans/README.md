# Incular memory architecture plans

Created 2026-09-26. Plans [01](01-auxiliary-state-results.md), [02](02-semantic-metadata-results.md), [03](03-widget-textfield-results.md) and [04](04-text-cache-key-results.md) have measured implementations; remaining ownership census/lifecycle work, semantic occupancy census, larger enum layouts and borrowed-key lookup are open, respectively. A further [render text-field candidate](03-render-textfield-rejected.md) was measured and reverted because resident memory increased. Plan [05](05-single-run-glyph-sharing-results.md) now shares one-run glyph storage, while mixed-run/document storage remains open. A related [font-run duplication fix](06-font-run-duplication-results.md) is also recorded, but the rest of plan 06 is open. Plans 07–10 remain proposed. No optimization plan is fully complete yet.

Goal: reduce real application memory toward below 80 MB without removing features,
changing pixels, or increasing idle CPU. The newest strict result is 103.76 MB
private resident memory, 136.22 MB private commit and 0% sampled idle CPU;
the calculated two-file bundle is 11.01 MB. See [the latest DX12 result](../DESCRIPTOR-LIMIT.md),
[earlier baseline evidence](../MEMORY-DISK.md) and
[graphics investigation](../MEMORY-80.md).

## Plans and execution order

| Plan | Initial priority | Main concern |
| --- | --- | --- |
| [00: Measurement protocol](00-measurement.md) | Prerequisite | Attribute savings accurately |
| [01: Specialized element state](01-element-state.md) | Scrolling and auxiliary state measured; census open | Lifecycle and subscription ownership |
| [02: Optional semantic metadata](02-semantic-metadata.md) | Implementation measured; occupancy census open | Accessibility and descriptor isolation |
| [03: Compact enum payloads](03-enum-layout.md) | Render features and TextField widget measured; larger enums open | Allocation overhead and public APIs |
| [04: Single-owner text cache keys](04-text-cache-keys.md) | First patch measured | Equality and eviction correctness |
| [05: Shared glyph storage](05-glyph-storage.md) | One-run sharing measured; mixed/document storage open | Coordinates, editing and public APIs |
| [06: Shared font diagnostic strings](06-font-debug-strings.md) | Early isolated change | Diagnostic/API compatibility |
| [07: Immutable text ownership](07-immutable-strings.md) | After ownership census | Sharing versus allocation overhead |
| [08: Text cache byte budget](08-text-cache-budget.md) | After 04 | Cache thrashing and accounting |
| [09: Arena reclamation](09-arena-reclamation.md) | Later | Stable and stale identity correctness |
| [10: Transient allocation recovery](10-transient-capacity.md) | Separate resource track | GPU lifetimes and idle wakeups |

Start with 00. Prefer small 04/06 patches and structural 01/02 patches first,
then 03/07, then 08. Undertake 05/09 only where measurements justify the larger
change. Reassess ordering after the census. Plan 10 is independently scoped;
this is not an instruction to start parallel agents.

## Architecture and compatibility

Follow [the architecture contract](../../../system-design/ARCHITECTURE.md) and
[dependency/support specification](../../../specs/architecture.json). Maintain
one behavioral owner, one reactive engine and one desktop host. Keep neutral
storage outside WGPU and native APIs in native owners. Resolve symbols again
before editing if a concurrent ownership migration has moved their files.

Preserve application builders. Public fields/types require an API-class audit
and a documented migration before representation changes. `doc(hidden)` does
not make public Rust types private. Justify new dependencies by measured benefit
and bundle cost. Do not use unsafe packed representations or narrow IDs without
preserving their valid range and generation semantics.

## Validation and completion

Each plan names focused regressions. Confirm current Cargo targets/features
before running them. Keep tests and test-only helpers under `tests/`. Run targeted
tests/checks per patch, not the full suite repeatedly. At the completed integration
batch, run the broader repository checks warranted by its scope and architecture
changes, honoring the user's request to avoid unnecessary full tests. Finish with
an appropriate compiler error check.

Every plan delivers an independently revertible patch, a before/after measurement,
targeted validation, and an accepted/rejected/inconclusive decision. Keep only
demonstrated wins. Never add overlapping estimates together: measure the combined
build. No estimated MB savings are promised before counting actual objects and
allocations.

## Research and limits

Cloudflare's DNS-cache work used fixed-size ownership, fewer buffers, less
duplicate data and compact enum representations. Its 100 TB saving was fleet-wide;
the techniques apply where Incular's object counts and ownership justify them.
[Cloudflare's engineering article](https://blog.cloudflare.com/dns-cache-memory-optimization-1111/).

The graphics submission/presentation investigation remains separate. The triangle
probe is not a proven memory floor, and struct changes are not a promise to close
the entire 29.54 MB gap. These plans do not authorize driver installation or a
renderer replacement. All existing capabilities and the fixed workload remain.
