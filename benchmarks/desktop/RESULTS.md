# Latest desktop benchmark results

Windows AMD Radeon 610M, three idle-qualified launches each. Decimal MB; CPU is percent of one core. Incular uses the latest distribution build; Electron is the retained previous measurement on this host and was not rerun in this experiment.

| Application | Private resident MB | Private committed MB | Idle CPU | Qualified launches |
| --- | ---: | ---: | ---: | ---: |
| Incular | **94.70** | **114.50** | 0.00% | 3/3 |
| Incular, previous result | 103.76 | 136.22 | 0.00% | 3/3 |
| Electron | 104.40 | 196.98 | 0.00% | 3/3 |

The Incular executable plus the VC runtime DLL totals **10.31 MB** (10,305,976 bytes). The retained Electron installed bundle is 386.14 MB. The three Incular launches ranged from 94.33 to 95.19 MB private resident.

Incular is now 9.70 MB below the retained same-host Electron result; Electron was not rerun. On this AMD integrated GPU, a bare native D3D12 window measured about 84.6 MB ([AMD-HARDWARE.md](AMD-HARDWARE.md)), and most of the remaining private memory is driver-owned. QuickGUI's macOS chart uses a different OS and memory metric, so it is not a same-machine comparison.

Changes since the previous result, all with unchanged features, backends and pixels (five captured states identical):

- `Container` no longer clips by default, so the full-window stencil buffer is only allocated for frames that draw rounded or path clips.
- System fonts are shared memory maps instead of private copies, and first-frame glyph uploads are no longer padded per glyph.
- Paint writes straight into retained compositor layers instead of assembling a discarded display list; layers, render objects and widget payloads are smaller.
- Tokio starts on first use (47 to 15 idle threads), semantics are built only while assistive technology listens, text layouts derive caret stops on demand, and retained arenas grow by half instead of doubling.
- Rare large render, widget and feature payloads are boxed (a render kind is 96 instead of 152 bytes), and display lists grow from one command instead of four.
- Frames skip leader-link publication when no leader layers exist, look up diagnostic names only when reporting, and prune action handlers only after bindings change.
- The distribution profile uses fat LTO.

Rust heap at idle fell from 12.41 MB live / 16.51 MB peak to 5.92 MB / 6.39 MB.

- [Hardware findings and methodology](AMD-HARDWARE.md)
- [Paired memory and disk reductions](MEMORY-DISK.md)
- [Latest text cache key result](memory-plans/04-text-cache-key-results.md)
- [Retained scrolling-state result](memory-plans/01-scrolling-sidecar-results.md)
- [Boxed render-feature result](memory-plans/03-render-feature-results.md)
- [Boxed text-field widget result](memory-plans/03-widget-textfield-results.md)
- [Latest optional auxiliary element-state result](memory-plans/01-auxiliary-state-results.md)
- [Latest optional semantic metadata result](memory-plans/02-semantic-metadata-results.md)
- [Latest DX12 descriptor-limit result](DESCRIPTOR-LIMIT.md)
- [Single-run glyph-sharing result](memory-plans/05-single-run-glyph-sharing-results.md)
- [Bundle reduction and dependency patches](BUNDLE-SIZE.md)
- [Retained compact evidence and validation](results/README.md)
- [Electron visual comparison](results/bundle-dist/visuals/compare-issue-tracker.png)
