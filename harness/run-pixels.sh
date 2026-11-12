#!/bin/sh
# Pixel parity: captures every scene of harness/flutter/lib/main.dart with
# Flutter's Windows (Skia) raster, then renders the same scenes through
# Incular's WGPU renderer and diffs them. Needs a Flutter SDK with Windows
# desktop support and a GPU window (set FLUTTER when `flutter` is not on PATH).
#
# Usage: harness/run-pixels.sh [--incular-only]
# Output: harness/out/pixels/{flutter,incular,diff}; report on stderr.
# Diagnostics: INCULAR_PARITY_PROBE, INCULAR_PARITY_BANDS, INCULAR_PARITY_TONE
# (see harness/incular/tests/pixels.rs).
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
out=${INCULAR_PARITY_DIR:-$root/harness/out}
flutter=${FLUTTER:-flutter}
mkdir -p "$out/pixels/flutter"
if [ "$1" != "--incular-only" ]; then
  (cd "$root/harness/flutter" && PARITY_OUT="$out/pixels/flutter" "$flutter" run -d windows --release)
fi
cd "$root"
INCULAR_PARITY=1 INCULAR_PARITY_DIR="$out" \
  cargo test -q -p incular-parity-harness --test pixels
