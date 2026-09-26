# Boxed render text-field candidate rejected

Status: measured and reverted on 2026-09-26. Plan 03 remains open for a variant
occupancy census and other layouts.

The candidate moved the full text-field configuration out of `RenderKind`'s
inline storage. On the 64-bit build, `RenderKind` fell from **352 to 264 bytes**
and every common `RenderNode` from **560 to 472 bytes**. Focused editable-text,
bidi, alignment, input and rendering tests passed (52 tests). All five issue
tracker PNGs matched the preceding distribution build byte for byte.

| Same-host strict idle run | Private resident median | Three-launch range | Private commit median | Idle CPU | Executable |
| --- | ---: | ---: | ---: | ---: | ---: |
| Prior source rebuilt in an isolated checkout | 105.72 MB | 105.72–105.80 MB | 143.42 MB | 0% | 10,827,264 B |
| Boxed render text-field | 106.24 MB | 105.99–106.35 MB | 141.43 MB | 0% | 10,829,824 B |

The candidate's private resident median was **520,192 bytes higher**. The
ranges did not overlap, and the earlier published prior-source run was
105.67 MB, close to the rebuilt baseline. The isolated rebuild produced a
different executable hash and size, so this comparison does not prove a
universal causal regression from boxing. It does show no measured resident
benefit on the target workload. The candidate added one allocation for each
text-field render configuration and 2,560 executable bytes versus the paired
rebuild. Following plan 03's acceptance rule, the source change was reverted.

[Compact paired evidence](../results/memory-architecture/03-render-textfield-rejected.json).
