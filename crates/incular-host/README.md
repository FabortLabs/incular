# incular-host

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Shell-independent window presentation, retry pacing and GPU diagnostics shared by every host shell. |
| API class | backend; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Used by the Winit shell (desktop, web) and the mobile embedder shell. |

Host core for Incular. A `HostSurface` owns one window's WGPU renderer, the
metrics its content was laid out for, and the `PresentationRetry` pacing for
skipped frames. `HostSurface::present` renders a display list and settles
presentation, render metrics and simulation captures with the runtime.

This crate has no Winit, raw-window-handle or operating-system dependency.
Shells supply an `incular_wgpu::WindowSurfaceTarget`, native events and frame
ticks; see the [mobile spec](https://github.com/FabortLabs/incular/blob/master/system-design/MOBILE.md)
for how the Winit and embedder shells share it.
