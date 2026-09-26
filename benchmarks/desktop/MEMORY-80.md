# Investigation of the 80 MB target

Status: **not achieved**. The latest strict three-launch result is **105.67 MB private resident memory**, **142.27 MB private commit**, and **0% sampled idle CPU**. The calculated two-file bundle is **11.01 MB**. These are decimal MB. See [the optional auxiliary element-state result](memory-plans/01-auxiliary-state-results.md) for the newest measurement and [MEMORY-DISK.md](MEMORY-DISK.md) for the earlier paired baseline.

## Measured graphics baseline

Short diagnostic probes on the same AMD Radeon 610M / DX12 machine, driver 32.0.21036.11002, produced:

| Isolated workload | Private resident MB | Private commit MB |
| --- | ---: | ---: |
| Native window | 2.17 | 2.87 |
| WGPU device | 39.65 | 59.60 |
| Surface configured | 32.67 | 52.31 |
| Triangle pipeline compiled | 33.01 | 52.92 |
| Triangle drawn and presented, 256 warmup frames | 93.96 | 116.91 |

Each row is a separate process. These short probes are **not** strict benchmark runs and do not establish an irreducible driver memory floor. The triangle warmup also uses a different surface size from the framework's tiny startup workaround. They localize a large increase to GPU submission/presentation rather than widget retention or shader compilation alone. All sampled idle CPU medians were zero.

The app's diagnostic allocator counted about **17.70 MB of live Rust heap allocations**. WGPU reported **13.04 MB live GPU resource allocations** within **18.55 MB reserved blocks**. These are different accounting domains and must not be added to, or subtracted from, resident RAM as if they were disjoint measurements. Native driver allocations are not counted by the Rust allocator.

Evidence: [GPU stages](results/memory-disk/under-80/gpu-stages.json), [configuration and compilation](results/memory-disk/under-80/gpu-configure-compile.json), [heap and resource snapshots](results/memory-disk/under-80/heap-and-resources.log).

### Presentation isolated from drawing (2026-09-26)

The updated standalone probe uses the same DX12 adapter, 65,536 non-sampler
bindings and small initial WGPU blocks as the application. Each row below is a
separate process after 256 startup frames, except configure/compile, which stop
before presentation. The native window was 1650 × 1080 pixels. All sampled idle
CPU medians were zero.

| Isolated workload | Frame latency | Private resident MB | Private commit MB |
| --- | ---: | ---: | ---: |
| Surface configured | 2 | 32.77 | 51.29 |
| Triangle pipeline compiled | 2 | 33.14 | 54.58 |
| Triangle drawn to an offscreen texture, no surface acquire/present | 1 | 46.46 | 79.19 |
| Surface clear and present, Mailbox | 1 | 84.68 | 108.85 |
| Triangle draw and present, Mailbox | 1 | 85.69 | 107.18 |
| Triangle draw and present with `COPY_SRC`, Mailbox | 1 | 85.76 | 108.38 |
| Surface clear and present with `COPY_SRC`, FIFO | 1 | 84.72 | 108.68 |
| Triangle draw and present with `COPY_SRC`, FIFO | 1 | 85.73 | 108.12 |
| Surface clear and present, Mailbox | 2 | 88.13 | 110.91 |

The approximately 38 MB gap between offscreen drawing and clear-only
presentation isolates a large cost to WGPU/DX12 surface presentation on this
machine; drawing itself adds about 1 MB to the presented process. The
application already requests one frame of latency, which saved about 3.45 MB
versus two in this probe. Switching Mailbox to FIFO did not reduce memory. A
256-frame acquired-but-unpresented loop stalled and did not yield a valid
memory result. These short process measurements are **not** strict app results
or proof that native D3D12 has the same cost; a native D3D12 control remains
necessary before attributing it to the AMD driver or WGPU itself.

Evidence: [presentation isolation](results/memory-architecture/gpu-present-isolation/gpu-presentation-isolation.json).
[WGPU's surface documentation](https://docs.rs/wgpu/30.0.0/wgpu/type.SurfaceConfiguration.html)
describes one-frame latency as a GUI choice and maps it to DXGI maximum frame
latency; [Microsoft's D3D12 swapchain documentation](https://learn.microsoft.com/en-us/windows/win32/direct3d12/swap-chains)
describes the required flip-model presentation path.

## Experiments rejected

- GL backend: approximately 198 MB resident, worse than DX12.
- Vulkan backend: approximately 188 MB resident in a short same-app probe, worse than DX12; this is not a strict result.
- DX12 DirectComposition presentation: approximately 110 MB resident.
- Draining the AMD startup workaround after every present: no meaningful resident improvement.
- Reusing the startup swapchain: approximately 109 MB resident; reverted.
- Lazy window stencil attachment: this benchmark still requires a stencil attachment; reverted with its pipeline variants.
- Segment Heap manifest in a separate executable copy: approximately 109 MB resident and 143 MB commit. No shipping manifest change; the resident target was not improved. [Microsoft heapType documentation](https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests#heaptype).

No feature, workload, rendering backend, or AMD idle-CPU workaround was removed. No working-set trimming was used. The shipping package and published strict result were not replaced by an experimental binary.

## Reproduction tools

The diagnostic-only `issue_tracker_memory_profile` example wraps the unchanged issue tracker with Rust allocation counters. `INCULAR_HEAP_STAGE=empty` or `text` isolates non-GPU startup; omit it for the full workload. It exits after 30 seconds and is not a process-memory benchmark executable.

The opt-in `wgpu_idle_probe` supports `INCULAR_GPU_PROBE_BINDINGS=65536`, `INCULAR_GPU_PROBE_SMALL_ALLOCATIONS=1`, `INCULAR_GPU_PROBE_LATENCY=1`, `INCULAR_GPU_PROBE_FIFO=1`, and `INCULAR_GPU_PROBE_WARMUP=256`. The `draw-offscreen-only` stage submits a triangle without acquiring or presenting a surface frame. Run `probe-gpu-stages.py` with the built probe and `--backends dx12` to collect short stage samples. Use `INCULAR_GPU_PROBE_DRAIN=1` and `INCULAR_GPU_PROBE_PACE_MS=0` to reproduce the presentation table; configure/compile stop before warmup.

The next useful experiment is a same-machine comparison of WGPU presentation/submission against a minimal native D3D12 implementation or a newer AMD driver. That can distinguish WGPU overhead from driver behavior before making a larger renderer change. The public GPUI figure is not a same-machine backend comparison.
