# incular-parity-harness

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Frame-by-frame and pixel parity harnesses comparing Incular with Flutter. |
| API class | application; this package is a tool, not a framework re-export surface. |
| Support | Development tool; opt-in runs need a Flutter SDK, and the pixel harness a GPU window. |

The Incular side of the Flutter parity harnesses: the frame recorder,
comparer and report in `src/lib.rs`, frame scenarios in `tests/frames/`
and pixel scenes in `tests/pixels.rs`. The Flutter side and the workflow
are described in [`harness/README.md`](../README.md).
