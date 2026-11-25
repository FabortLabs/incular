#!/usr/bin/env bash
# Compare one to five exact existing frame scenarios on the pinned SDK.
set -euo pipefail
if (( $# == 0 || $# > 5 )); then
  echo "Usage: $0 SCENARIO [SCENARIO ...]" >&2
  exit 2
fi
audit="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$audit/../.." && pwd)"
flutter_root="$root/.tmp/flutter"
flutter="$flutter_root/bin/flutter"
if [[ "$(git -C "$flutter_root" rev-parse HEAD)" != ab2912035e160d9cad6306df307576da6128ebdb ]]; then
  echo 'Wrong Flutter reference revision.' >&2
  exit 2
fi
if [[ ! -x "$flutter_root/bin/cache/dart-sdk/bin/dart" ]]; then
  echo 'Bootstrap the pinned Flutter SDK before running the frame comparison.' >&2
  exit 2
fi

case_map=$(python3 - "$audit/existing-harness-scenarios.jsonl" "$@" <<'PY'
import json
import re
import sys

path, *names = sys.argv[1:]
if len(set(names)) != len(names):
    raise SystemExit("Duplicate scenario")
with open(path, encoding="utf-8") as f:
    catalog = [json.loads(line) for line in f]
for case in names:
    if re.fullmatch(r"[a-z0-9_]+", case) is None:
        raise SystemExit(f"Invalid scenario: {case}")
    description = case
    if case in {"scroll__drag_overscroll__clamping", "scroll__drag_overscroll__bouncing"}:
        description = "'scroll__drag_overscroll__$physics'"
    matches = [
        row["file"]
        for row in catalog
        if row["registration"] == "frameScenario" and row["description"] == description
    ]
    if len(matches) != 1 or not re.fullmatch(r"harness/flutter/test/frames/[a-z0-9_]+_test\.dart", matches[0]):
        raise SystemExit(f"No unique tracked Flutter frame fixture for {case}")
    print(f"{case}\t{matches[0].removeprefix('harness/flutter/')}")
PY
)

scratch="$root/.tmp/flutter350-full-audit/frame-batches"
mkdir -p "$scratch"
out="$(mktemp -d "$scratch/linux-XXXXXXXX")"
echo "Focused frame batch: $out"
mkdir -p "$out/reference" "$out/frames/flutter"
cp -a "$root/harness/flutter/"{pubspec.yaml,fonts,test} "$out/reference/"
# The isolated test package does not run the analyzer. Do not require
# the lint-only package in an offline cache or change the source workspace.
sed -i '/^  flutter_lints:/d' "$out/reference/pubspec.yaml"
export CI=true FLUTTER_SUPPRESS_ANALYTICS=true
export INCULAR_PARITY=1 INCULAR_PARITY_DIR="$out" PARITY_SCENARIO_EXACT=1 PARITY_STRICT=1
export PARITY_FRAMES_OUT="$out/frames/flutter"

if ! (cd "$out/reference" && "$flutter" pub get --offline) > "$out/pub.log" 2>&1; then
  cat "$out/pub.log" >&2
  exit 1
fi
failed=0
while IFS=$'\t' read -r scenario file; do
  if ! (
    cd "$out/reference"
    "$flutter" test --no-pub --concurrency=1 --reporter expanded --name "^$scenario$" "$file"
  ) > "$out/$scenario-flutter.log" 2>&1 || [[ ! -f "$PARITY_FRAMES_OUT/$scenario.json" ]]; then
    printf '%s: reference failed\n' "$scenario" >&2
    failed=1
    continue
  fi
  if ! (
    cd "$root"
    PARITY_SCENARIO="$scenario" cargo test --locked -p incular-parity-harness -j 1 \
      --test frames -- --test-threads=1
  ) > "$out/$scenario-incular.log" 2>&1; then
    failed=1
  fi
  report="$out/frames/report-$scenario.md"
  if [[ ! -f "$report" ]] || ! grep '^## .* — match (' "$report"; then
    [[ ! -f "$report" ]] || grep '^## ' "$report"
    printf '%s: inspect its comparison report or log\n' "$scenario" >&2
    failed=1
  fi
done <<< "$case_map"
exit "$failed"
