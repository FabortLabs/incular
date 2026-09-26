# Retained scrolling state result

Status: first family of [plan 01](01-element-state.md) implemented and measured on 2026-09-26. Notification, editing and layout-builder fields remain inline; the full plan is open.

`Element` previously embedded four scrolling identity collections and three revision counters in every node. These are now one lazily allocated, private `ElementScrollingState`. Ordinary nodes keep a nullable pointer; sliver, wheel and advanced-scrolling nodes acquire the record when their state is first written. Existing read paths return empty collections or zero revisions without allocating. Reconciliation retains state on compatible updates, and unmount drops it with its element.

The 64-bit common `Element` size fell from **400 to 264 bytes**. A 1,001-element ordinary-tree test therefore has **400,400 to 264,264 bytes** of shallow element storage, a **136,136-byte** reduction before arena capacity and allocator effects. Specialized scrolling nodes add a boxed record and allocation; these costs are included in the process benchmark, but the current app's scrolling-owner count and deep heap bytes were not separately instrumented.

| Same-machine strict idle run | Private resident median | Private commit median | Idle CPU | Executable |
| --- | ---: | ---: | ---: | ---: |
| Before, optional semantics | 107.49 MB | 145.79 MB | 0% | 10,824,192 B |
| Scrolling state sidecar | **107.00 MB** | **144.17 MB** | **0%** | 10,825,216 B |

The three new private-resident launch medians were 106.48, 107.00 and 107.05 MB, all below the previous launch range of 107.28–107.57 MB. The median improvement is **0.50 MB**; commit varied more across launches, so its 1.62 MB median difference should not be attributed solely to this change. Both results used the 1,000-record/100-row issue tracker, visible 1100×720 window, default DX12 backend and the same three-launch idle policy. All five captured states have identical PNG hashes. The calculated executable-plus-VC-runtime bundle is **11,003,832 bytes (11.00 MB)**, 1,024 bytes larger than before.

Focused `element_memory`, scrolling/sliver, advanced-scrolling and reconciliation tests passed (62 tests). See [paired machine-readable results](../results/memory-architecture/01-scrolling-sidecar.json). This is an accepted incremental saving, not completion of plan 01 or the below-80-MB goal. The next field families need an owner census and separate measurements.
