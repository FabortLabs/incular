#!/bin/sh
# Frame-by-frame parity: records every scenario in Flutter, then in Incular,
# and compares them. Run from anywhere; needs a Flutter SDK (set FLUTTER to
# its `flutter` executable when it is not on PATH).
#
# Usage: harness/run-frames.sh [scenario-name-substring]
# Report: harness/out/frames/report.md (report-<substring>.md when filtered)
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
out=${INCULAR_PARITY_DIR:-$root/harness/out}
flutter=${FLUTTER:-flutter}
mkdir -p "$out/frames/flutter"
(
  cd "$root/harness/flutter"
  if [ -n "$1" ]; then
    PARITY_FRAMES_OUT="$out/frames/flutter" "$flutter" test test/frames --plain-name "$1"
  else
    PARITY_FRAMES_OUT="$out/frames/flutter" "$flutter" test test/frames
  fi
)
cd "$root"
INCULAR_PARITY=1 INCULAR_PARITY_DIR="$out" PARITY_SCENARIO="$1" \
  cargo test -q -p incular-parity-harness --test frames
