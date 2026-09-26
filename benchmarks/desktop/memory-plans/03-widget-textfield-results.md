# Box the uncommon text-field widget payload

Status: accepted second implementation of [plan 03](03-enum-layout.md), measured after [private render-feature compaction](03-render-feature-results.md) on 2026-09-26. Other `WidgetKind` variants and the public `RenderKind` remain open.

The private `WidgetKind` stored a 400-byte `TextFieldSpec` inline, making every `WidgetNode` reserve that space. Only the `TextField` variant now owns a boxed spec. Cloning, mutation and field access keep their existing behavior, while text fields pay for one additional allocation.

| 64-bit retained type | Before | After | Shallow saving |
| --- | ---: | ---: | ---: |
| `WidgetKind` | 400 B | 280 B | 120 B |
| `WidgetNode` | 432 B | 312 B | 120 B |

The exact desktop widget count and text-field occupancy were not separately instrumented, so the shallow saving is not multiplied into an assumed application total.

| Same-machine strict idle run | Private resident median | Launch range | Private commit median | Idle CPU | Executable |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before, compact render feature state | 106.79 MB | 106.64–107.41 MB | 144.57 MB | 0% | 10,825,216 B |
| Boxed text-field widget payload | **105.98 MB** | **105.37–106.08 MB** | **144.27 MB** | **0%** | 10,826,240 B |

All three new resident launch medians fall below the previous launch range. The median reduction is **811,008 bytes (0.81 MB)** under the same 1,000-record/100-retained-row visible Windows workload and idle policy. All five captured visual states have identical PNG hashes. The calculated executable-plus-VC-runtime bundle is **11,004,856 bytes (11.00 MB)**, 1,024 bytes larger than before. No capability or rendering path was removed.

Focused lifecycle, editable-text, editing-interaction and reconciliation tests passed (34 tests). See [paired machine-readable results](../results/memory-architecture/03-boxed-textfield-widget.json). Plan 03 remains open: further variant changes require an occupancy histogram, allocation accounting and an API audit for public `RenderKind`.
