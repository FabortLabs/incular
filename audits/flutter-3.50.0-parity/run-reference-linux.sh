#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf '%s\n' 'Usage: bash run-reference-linux.sh probe FILE_test.dart' \
    '       bash run-reference-linux.sh upstream LIBRARY/FILE_test.dart "Exact full test name" [...]' >&2
  exit 2
}

(( $# >= 2 )) || usage
mode=$1
test_file=$2
shift 2
audit=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
root=$(cd -- "$audit/../.." && pwd -P)
flutter_root="$root/.tmp/flutter"
revision=ab2912035e160d9cad6306df307576da6128ebdb
if [[ $(git -C "$flutter_root" rev-parse HEAD) != "$revision" ]]; then
  printf 'Expected pinned Flutter checkout %s\n' "$revision" >&2
  exit 1
fi

case "$mode" in
  probe)
    (( $# == 0 )) || usage
    [[ "$test_file" =~ ^[a-z0-9_]+_test\.dart$ ]] || usage
    fixture="$audit/reference-probes/test/$test_file"
    ;;
  upstream)
    (( $# >= 1 && $# <= 5 )) || usage
    [[ "$test_file" =~ ^(animation|foundation|gestures|painting|physics|rendering|scheduler|semantics|services|widget_previews|widgets)/([a-z0-9_]+/)*[a-z0-9_]+_test\.dart$ ]] || usage
    fixture="$flutter_root/packages/flutter/test/$test_file"
    pattern=$(python3 - "$@" <<'PY'
import re
import sys

names = sys.argv[1:]
if any(not name.strip() for name in names):
    raise SystemExit("Give exact test names, including group prefixes.")
# Repeated names select distinct registrations with the same full name.
# The result check below still requires an exact count for each name.
print("^(?:" + "|".join(map(re.escape, dict.fromkeys(names))) + ")$")
PY
    )
    ;;
  *) usage ;;
esac
[[ -f "$fixture" ]] || { printf 'Unknown test: %s\n' "$fixture" >&2; exit 1; }

scratch_root="$root/.tmp/flutter350-full-audit"
mkdir -p "$scratch_root"
scratch=$(mktemp -d "$scratch_root/linux-${mode}-XXXXXXXX")
if [[ "$mode" == probe ]]; then
  mkdir "$scratch/test"
  cp -- "$audit/reference-probes/pubspec.yaml" "$scratch/pubspec.yaml"
  cp -- "$fixture" "$scratch/test/$test_file"
  fixture="test/$test_file"
else
  python3 - "$flutter_root/packages/flutter_goldens" > "$scratch/pubspec.yaml" <<'PY'
import json
import sys

print("""name: incular_upstream_reference_probe
publish_to: none
environment:
  sdk: ^3.13.0-0
dependencies:
  flutter:
    sdk: flutter
dev_dependencies:
  flutter_test:
    sdk: flutter
  flutter_goldens:
    path: """ + json.dumps(sys.argv[1]))
PY
fi

printf 'Pinned reference run: %s\n' "$scratch"
export CI=true FLUTTER_SUPPRESS_ANALYTICS=true
flutter="$flutter_root/bin/flutter"
pub_args=(--offline)
if [[ ${INCULAR_BOOTSTRAP_PUB:-0} == 1 ]]; then
  pub_args=()
fi
cd "$scratch"
if ! "$flutter" pub get "${pub_args[@]}" > pub.log 2>&1; then
  cat pub.log >&2
  exit 1
fi

test_args=(--no-pub --concurrency=1 --reporter expanded)
if [[ "$mode" == upstream ]]; then
  test_args+=(--name "$pattern")
fi
"$flutter" test "${test_args[@]}" "$fixture" 2>&1 | tee result.log
if [[ "$mode" == upstream ]]; then
  python3 - "$@" <<'PY'
from collections import Counter
from pathlib import Path
import re
import sys

reported = []
for line in Path("result.log").read_text(encoding="utf-8").splitlines():
    match = re.fullmatch(r"\d+:\d+ \+\d+: (.+)", line)
    if not match:
        continue
    name = match[1]
    if name.startswith("loading ") or name in {"All tests passed!", "(setUpAll)", "(tearDownAll)"}:
        continue
    reported.append(name)
if Counter(reported) != Counter(sys.argv[1:]):
    raise SystemExit(f"Selected test names do not match the reporter: {reported!r}")
PY
fi
