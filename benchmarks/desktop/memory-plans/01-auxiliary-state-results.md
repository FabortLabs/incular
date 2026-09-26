# Optional auxiliary element state

Status: accepted second implementation of [plan 01](01-element-state.md), measured after [scrolling-state specialization](01-scrolling-sidecar-results.md) on 2026-09-26. Ownership census and further lifecycle/reclamation analysis remain open.

`Element` previously stored notification subscriptions, editing subscriptions, and layout-builder constraints/revision inline even for ordinary nodes. These independent behaviors now share one optional private record; they can coexist on a node. Reading default layout-builder state and clearing absent subscriptions do not allocate. Existing RAII subscriptions are still dropped before registrations are rebuilt, and the record is dropped on unmount.

The 64-bit common `Element` size fell from **264 to 168 bytes**. In the 1,001-element ordinary-tree test, shallow element storage fell from **264,264 to 168,168 bytes**, a **96,096-byte** reduction before arena capacity and allocator effects. Active owners pay for one boxed record and its allocation. Their count and deep heap bytes in the desktop app were not separately measured.

| Same-machine strict idle run | Private resident median | Launch range | Private commit median | Idle CPU | Executable |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before, boxed text-field widget | 105.98 MB | 105.37–106.08 MB | 144.27 MB | 0% | 10,826,240 B |
| Optional auxiliary state | **105.67 MB** | **105.58–105.75 MB** | **142.27 MB** | **0%** | 10,828,288 B |

The **0.31 MB** lower resident median is inside overlapping launch ranges, so its process-level effect is **inconclusive**. The 96-byte common-record saving is deterministic and accepted. The same visible 1,000-record/100-row Windows workload, backend and idle policy were used; all five captured states have identical PNG hashes. The calculated executable-plus-VC-runtime bundle is **11,006,904 bytes (11.01 MB)**, 2,048 bytes larger than before.

Focused editing, inherited-context, reconciliation, scrolling and element-size tests passed (63 tests). See [paired machine-readable results](../results/memory-architecture/01-optional-auxiliary-state.json). Plan 01 remains open for actual owner counts, deep allocations, peak rebuild overlap and incompatible-kind reclamation; this result does not claim those measurements.
