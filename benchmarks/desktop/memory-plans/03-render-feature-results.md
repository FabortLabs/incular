# Compact private render feature payloads

Status: first implementation of [plan 03](03-enum-layout.md), measured on 2026-09-26. A [text-field widget follow-up](03-widget-textfield-results.md) is measured separately; other `WidgetKind` and public `RenderKind` layout work remains open.

Compiler type-layout diagnostics on 64-bit Windows identified `RenderTwoDimensionalState` (176 bytes), `RenderWheelState` (120 bytes) and `RenderRawScrollbarState` (88 bytes) as the three largest `RenderFeatureState` payloads. They are now boxed only for their corresponding feature variants. The issue-tracker workload does not construct these three kinds; other widgets retain inline text, button, scroll and editing state. Existing state reconciliation and controller lifetimes are unchanged.

| Retained type | Before | After | Shallow saving |
| --- | ---: | ---: | ---: |
| `RenderFeatureState` | 176 B | 40 B | 136 B |
| `RenderNode` | 696 B | 560 B | 136 B |

Specialized nodes now pay for a box allocation. The exact number of retained render nodes and deep heap bytes in the desktop app were not separately instrumented, so the structural saving is not converted into an app-wide byte estimate.

| Same-machine strict idle run | Private resident median | Launch range | Private commit median | Idle CPU | Executable |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before, scrolling sidecar | 107.00 MB | 106.48–107.05 MB | 144.17 MB | 0% | 10,825,216 B |
| Boxed render features | **106.79 MB** | **106.64–107.41 MB** | **144.57 MB** | **0%** | 10,825,216 B |

The 0.20 MB resident median decrease is smaller than the overlapping launch variation, so the process-level improvement is **inconclusive**. The type-layout saving is deterministic and accepted. The workload, window, backend and idle policy match the preceding run; all five screenshot hashes are identical. The calculated two-file bundle remains **11,003,832 bytes (11.00 MB)**. The executable hash changed while its size did not.

Focused advanced scrolling, two-dimensional/scrollbar, wheel, editable text and rendering tests passed (83 tests). The 64-bit type-layout ceilings now guard against accidental inline growth. See [paired machine-readable results](../results/memory-architecture/03-boxed-render-features.json). Further enum work requires a variant histogram and net allocation measurement before selecting `WidgetKind` or public `RenderKind` representation changes.
