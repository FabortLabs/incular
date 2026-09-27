# Smaller default DX12 descriptor reservation

Implemented 2026-09-27. [Compact paired evidence](results/memory-architecture/gpu-descriptor-limit.json).

WGPU's DX12 backend allocates a shader-visible heap sized by `max_non_sampler_bindings` at device creation. Incular previously requested 65,536 live bindings, after already reducing WGPU's one-million default. An isolated same-host WGPU probe showed that 8,192 bindings reduced private resident memory by about 1.7 MB during presentation compared with 65,536; 2,048 bought only another 0.25 MB. The full UI now defaults to 8,192. Applications retaining more simultaneous GPU bindings can set `INCULAR_GPU_MAX_NON_SAMPLER_BINDINGS` to a larger positive `u32` value before startup; 65,536 was exercised with the unchanged interaction smoke. This is a device-wide live-binding budget, not a restriction on image dimensions, glyph count, effects or widget types. Invalid override values use the default; unsupported values fail device creation.

Same visible 1,000-record/100-row issue tracker, AMD Radeon 610M driver 32.0.21036.11002, DX12, three strict idle-qualified launches of each distribution executable:

| Build | Private resident median | Run range | Private commit median | Idle CPU | EXE bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 65,536 bindings | 105.40 MB | 104.89–105.67 MB | 140.84 MB | 0% | 10,827,776 |
| 8,192 bindings with override available | **103.76 MB** | **103.17–103.85 MB** | **136.22 MB** | **0%** | 10,828,288 |
| Same-host Electron 44.2.0, retained September 25 result | 104.40 MB | See linked result | 196.98 MB | 0% | — |

The two Incular resident ranges do not overlap: the observed reduction is **1.64 MB**. The new Incular result is **0.64 MB below the retained same-host Electron measurement**, though Electron was not rerun on September 27. The calculated Incular EXE plus VC runtime is 11,006,904 bytes (11.01 MB), versus Electron's retained 386.14 MB installed bundle; the Incular executable grew 512 bytes. Five UI-state PNG hashes match the validated baseline exactly. Default and high-capacity interaction smoke passed. This change leaves the below-80-MB goal unmet; the native DXGI/AMD presentation control alone still measured about 84.6 MB in a different, shorter workload.

These Windows private-resident figures cannot be compared directly with QuickGUI's macOS physical-footprint chart. An application that genuinely needs more than 8,192 concurrent non-sampler bindings must select a larger limit before creating its first GPU device; the DX12 heap does not grow automatically. No ongoing idle timer or rendering workaround changed.
