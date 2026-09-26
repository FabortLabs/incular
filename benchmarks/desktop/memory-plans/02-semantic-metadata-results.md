# Optional widget semantic metadata: measured result

Implemented 2026-09-26. [Compact paired evidence](../results/memory-architecture/02-optional-semantic-metadata.json).

`WidgetNode` now stores `Option<Box<SemanticProperties>>`. Its 55 ordinary
constructors leave the metadata absent. A read of an absent record uses a local
default value; a builder allocates the full record on its first actual change.
One custom-content constructor allocates only when it finds an accessible label.
Clone-on-write descriptor behavior and public accessibility builders remain.
The target compiler reported that an inline `SemanticProperties` default occupies
at least 416 bytes; the absent representation is a pointer-sized `Option<Box<_>>`.
That is a type-layout observation, not a direct process-memory saving per node.

Focus builders that receive default `false` or `None` avoid allocating when a
descriptor has no metadata. They still update an existing record, so a later
default value can clear an earlier nondefault value.

## Application measurement

Same AMD Radeon 610M host, visible 1100 × 720 issue tracker with 1,000 records
and 100 retained rows. Each binary had three strict idle-qualified launches.
Decimal MB; CPU is percent of one logical core.

| Distribution executable | Private resident median | Run range | Private commit median | Idle CPU |
| --- | ---: | ---: | ---: | ---: |
| Previous font-run fix, inline semantic record | 109.04 MB | 108.90–109.20 MB | 149.69 MB | 0% |
| Optional semantic record | 107.45 MB | 106.55–107.50 MB | 146.24 MB | 0% |
| Optional record with default-aware focus builders, current | **107.49 MB** | **107.28–107.57 MB** | **145.79 MB** | **0%** |

The observed difference from the preceding inline-record build to the current
build is **1.54 MB less private resident memory**. The two optional-record
variants differ by only 0.04 MB in median resident memory; that does not
establish a separate process-level saving from default-aware builders. The
current executable is 10,824,192 bytes, 8,704 bytes larger than the preceding
one. Adding the same 178,616-byte VC runtime DLL gives a calculated two-file
size of **11,002,808 bytes (11.00 MB)**. No package manifest was overwritten.

## Behavior and limits

Selected semantics, semantic-roots, focus and tree-reconciliation tests passed,
including a new test that mutating a cloned descriptor's label leaves its sibling
at the default. All five UI state PNG hashes are identical to the preceding
build. Current idle CPU samples remained zero. This is an app-level measurement;
it does not by itself count how many descriptors were default versus populated.
The next census should add those counts before tuning the representation further.

Decision: **retain**. The current app remains **27.49 MB above the 80 MB goal**.
