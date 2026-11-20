# Distribution bundle size

## Windows counter and issue tracker

Windows x86-64 MSVC, Rust 1.99.1, executable bytes on disk. Each comparison
uses the same Cargo profile and compiler settings for both applications. GPUI
is version 0.2.2. The Incular counter is the regular workspace example,
including its simulation support. Checks ran against each project's lockfile.

| Counter build | Incular bytes | GPUI bytes |
| --- | ---: | ---: |
| Standard release, previous Incular default | 20,790,272 | 11,185,664 |
| Standard release, current default | 14,740,992 | 11,185,664 |
| Distribution, current default | 9,739,776 | 4,852,224 |

The new default saves 6,049,280 bytes (29.1%) in the standard release.
With all native backends and bundled dictionaries enabled, the current
counter is 20,381,184 bytes. The issue tracker is 15,314,432 bytes with the
default features and 20,955,648 bytes with those full features. Initial and
post-increment window captures matched byte-for-byte across the two current
counter configurations on the test machine.

Reproduce the standard release from the workspace root:

```powershell
cargo build --locked --release -p incular --example counter --example issue_tracker
cargo build --locked --release --manifest-path .tmp/gpui-counter/Cargo.toml
```

The GPUI fixture lives in the local scratch directory. For the distribution
comparison, apply the workspace `[profile.dist]` settings to its standalone
manifest too. Both use optimization level `s`, fat LTO, one codegen unit and
symbol stripping. Sizes exclude PDBs, system DLLs and build intermediates.

## Earlier distribution measurements

The opt-in `dist` Cargo profile uses size optimization (`s`), fat LTO, one codegen unit and symbol stripping. The regular release profile remains available for throughput comparisons. Panic unwinding is preserved because the runtime catches callback panics. GPU backends and application features are unchanged.

```powershell
cargo build --profile dist -p incular --example issue_tracker --no-default-features --features desktop,controls
powershell -NoProfile -File benchmarks/desktop/package-windows.ps1 -Profile dist
python benchmarks/desktop/capture.py --executable target/desktop-benchmark/dist-bundle/issue_tracker.exe --states --output benchmarks/desktop/results/bundle-dist/visuals
```

Measured installed bytes include the executable and VC runtime DLL; PDB files are excluded from both builds. The system Arial fonts and Windows-provided components are excluded from both.

| Build | Installed bundle |
| --- | ---: |
| Previous release | 16.79 MB |
| Size-optimized distribution, before profiler separation | 12.24 MB |
| Distribution, profiler code excluded | 12.17 MB |
| Distribution, build-time shader parsing | 11.79 MB |
| Distribution, shared fonts and compressed dataset | 10.99 MB |
| Distribution, fat LTO | **10.31 MB** |

The current bundle is 10,992,056 bytes, approximately 34.5% smaller than the original release. Making GPU profiling truly optional saved 72,192 bytes. Moving built-in and WGPU-internal shader parsing to build time then saved another 375,296 bytes (3.1%) with the same distribution compiler settings and capabilities. See [shader implementation and validation](SHADER-PRECOMPILATION.md).

The latest change shares font source bytes and losslessly compresses the example's embedded dataset, reducing the installed bundle by another 797,696 bytes (6.8%). The exact 817,825-byte reference JSON is stored as 15,132 bytes of DEFLATE data; it is decoded once with a 1 MiB output limit, and the temporary buffer is dropped after parsing. No records or features were removed. This asset saving belongs to the benchmark application, not every Incular application. The normal library gains font source sharing. See [paired memory and disk measurements](MEMORY-DISK.md).

All five captured UI states are pixel-identical to the previous release; the initial capture clears hover deterministically. Exact bytes and hashes are in the [manifest](results/bundle-dist/bundle-manifest.json). The supplied chart lists QuickGUI Rust at 13.7 MB and GPUI at 6.3 MB on macOS; these are directional targets, not same-platform wins.

Validation of the shader changes included DX12/Vulkan/OpenGL and native-window regressions. The latest font and asset changes use focused font-cache/rasterization tests, an exact dataset round-trip test, a distribution build, five-state screenshot comparison and interaction smoke, formatting, workspace compilation and targeted Clippy. No full test suite was rerun. Current distribution memory measurements are recorded separately in [the paired report](MEMORY-DISK.md).

Fat LTO then saved about 0.66 MB over ThinLTO and made frame and layout benchmarks 6-11% faster, with the same `s` optimization level. `opt-level = "z"` would reach 9.19 MB but costs 18-39% CPU in those benchmarks, so it is not used.

GPU backends are Cargo features of the `incular` facade. These historical builds used the earlier all-backend desktop default. Current desktop builds select DX12 on Windows, Metal on macOS and Vulkan on Linux. Applications can add backends explicitly. Issue tracker executables with the `dist` profile (VC runtime DLL excluded):

| Backends | Executable |
| --- | ---: |
| DX12, Vulkan, OpenGL (default `desktop`) | 10.16 MB |
| DX12, Vulkan | 9.73 MB |
| DX12, OpenGL | 9.28 MB |
| Vulkan | 9.09 MB |
| DX12 | 8.83 MB |

The benchmark keeps the default backends.

Further size investigations should attribute linked code before changing capabilities: WGPU backend selection, text shaping/font fallback and image decoders. Optional GPU profiling has now been separated. Neither measured change establishes that Incular can reach the chart's GPUI size while retaining all native backends. No feature set was reduced for these measurements. Removing Vulkan fallback or supported codecs merely to lower this number would change the supported feature set.

[Cargo profile settings](https://doc.rust-lang.org/cargo/reference/profiles.html) document these build controls; size optimization is measured here rather than assumed to be smaller.
