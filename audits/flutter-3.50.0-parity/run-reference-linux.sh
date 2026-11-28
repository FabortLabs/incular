#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf '%s\n' 'Usage: bash run-reference-linux.sh probe FILE_test.dart' \
    '       bash run-reference-linux.sh upstream LIBRARY/FILE_test.dart "Exact full test name" [...]' \
    '       bash run-reference-linux.sh web-text-input services/text_input_test.dart "Exact full test name" [...]' \
    '       bash run-reference-linux.sh tooling WIDGET_PREVIEW_FILE_test.dart "Exact full test name" [...]' \
    '       bash run-reference-linux.sh scaffold WIDGET_PREVIEW_FILE_test.dart "Exact full test name" [...]' \
    '       bash run-reference-linux.sh scaffold-probe FILE_test.dart "Exact full test name" [...]' >&2
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
  upstream|web-text-input|tooling|scaffold|scaffold-probe)
    (( $# >= 1 && $# <= 5 )) || usage
    if [[ "$mode" == tooling ]]; then
      [[ "$test_file" =~ ^([a-z0-9_]+/)*[a-z0-9_]+_test\.dart$ ]] || usage
      tool_root="$flutter_root/packages/flutter_tools"
      case "$test_file" in
        migrations/widget_preview_gitignore_migration_test.dart)
          fixture="$tool_root/test/general.shard/$test_file"
          ;;
        preview_detector_regression_178472_test.dart|widget_preview_test.dart)
          fixture="$tool_root/test/commands.shard/permeable/widget_preview/$test_file"
          ;;
        *)
          fixture="$tool_root/test/commands.shard/hermetic/widget_preview/$test_file"
          ;;
      esac
    elif [[ "$mode" == scaffold || "$mode" == scaffold-probe ]]; then
      [[ "$test_file" =~ ^[a-z0-9_]+_test\.dart$ ]] || usage
      scaffold_root="$flutter_root/dev/integration_tests/widget_preview_scaffold"
      if [[ "$mode" == scaffold ]]; then
        fixture="$scaffold_root/test/$test_file"
      else
        fixture="$audit/reference-probes/scaffold/$test_file"
      fi
    else
      if [[ "$mode" == web-text-input && "$test_file" != services/text_input_test.dart ]]; then
        usage
      fi
      [[ "$test_file" =~ ^(animation|foundation|gestures|painting|physics|rendering|scheduler|semantics|services|widget_previews|widgets)/([a-z0-9_]+/)*[a-z0-9_]+_test\.dart$ ]] || usage
      fixture="$flutter_root/packages/flutter/test/$test_file"
    fi
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
elif [[ "$mode" == scaffold || "$mode" == scaffold-probe ]]; then
  cp -R -- "$scaffold_root/lib" "$scaffold_root/test" "$scratch/"
  if [[ "$mode" == scaffold-probe ]]; then
    cp -- "$fixture" "$scratch/test/$test_file"
  fi
  # Widget tests do not need the CLI's separate Dart process or its pinned
  # host-native dependencies. Fixtures importing CLI internals are not
  # runnable through this isolated widget-test package.
  if grep -q 'package:flutter_tools/' "$fixture"; then
    printf '%s\n' 'This fixture requires Flutter CLI tooling dependencies.' >&2
    exit 1
  fi
  python3 - "$scaffold_root/pubspec.yaml" > "$scratch/pubspec.yaml" <<'PYTHON'
from pathlib import Path
import sys

manifest = Path(sys.argv[1]).read_text(encoding="utf-8")
manifest = manifest.replace("resolution: workspace\n", "")
manifest = manifest.replace("  flutter_tools:\n    path: ../../../packages/flutter_tools/\n", "")
print(manifest)
PYTHON
  fixture="test/$test_file"
elif [[ "$mode" == upstream || "$mode" == web-text-input ]]; then
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
  if [[ "$mode" == web-text-input ]]; then
    # The browser compiler cannot import tests outside its package root.
    # Copy only this fixture and its pinned local helper/config files.
    source_root="$flutter_root/packages/flutter/test"
    mkdir -p "$scratch/test/services"
    cp "$fixture" "$source_root/services/text_input_utils.dart" "$scratch/test/services/"
    cp "$source_root/flutter_test_config.dart" "$source_root"/_goldens_{io,web}.dart "$scratch/test/"
    cmp "$fixture" "$scratch/test/$test_file"
    fixture="test/$test_file"
  fi
fi

printf 'Pinned reference run: %s\n' "$scratch"
export CI=true FLUTTER_SUPPRESS_ANALYTICS=true
flutter="$flutter_root/bin/flutter"
pub_args=(--offline)
if [[ ${INCULAR_BOOTSTRAP_PUB:-0} == 1 ]]; then
  pub_args=()
fi
cd "$scratch"
if [[ "$mode" == tooling ]]; then
  package_config="$tool_root/.dart_tool/package_config.json"
  if [[ ! -f "$package_config" ]]; then
    printf '%s\n' 'Initialize the pinned flutter_tools package and its Dart test dependencies first.' >&2
    exit 1
  fi
  runner=$(python3 - "$package_config" <<'CONFIG'
import json
from pathlib import Path
import sys
from urllib.parse import unquote, urlparse

config = Path(sys.argv[1])
for package in json.loads(config.read_text(encoding="utf-8"))["packages"]:
    if package["name"] == "test":
        uri = urlparse(package["rootUri"])
        path = Path(unquote(uri.path))
        print((path if uri.scheme else config.parent / path) / "bin/test.dart")
        break
else:
    raise SystemExit("Missing test package in the pinned flutter_tools package config")
CONFIG
  )
  (
    cd "$tool_root"
    export FLUTTER_ROOT="$flutter_root"
    "$flutter_root/bin/cache/dart-sdk/bin/dart" --packages="$package_config" "$runner" \
      --concurrency=1 --reporter expanded --name "$pattern" "$fixture"
  ) 2>&1 | tee result.log
else
  if ! "$flutter" pub get "${pub_args[@]}" > pub.log 2>&1; then
    cat pub.log >&2
    exit 1
  fi

  test_args=(--no-pub --concurrency=1 --reporter expanded)
  if [[ "$mode" == web-text-input ]]; then
    test_args=(--no-pub --platform chrome --reporter expanded)
  fi
  if [[ "$mode" == upstream || "$mode" == web-text-input || "$mode" == scaffold || "$mode" == scaffold-probe ]]; then
    test_args+=(--name "$pattern")
  fi
  "$flutter" test "${test_args[@]}" "$fixture" 2>&1 | tee result.log
fi
if [[ "$mode" != probe ]]; then
  python3 - "$@" <<'PY'
from collections import Counter
from pathlib import Path
import re
import sys

expected = Counter(sys.argv[1:])
reported = []
for line in Path("result.log").read_text(encoding="utf-8").splitlines():
    match = re.fullmatch(r"\d+:\d+ \+\d+: (.+)", line)
    if not match:
        continue
    name = match[1]
    if name.startswith("loading ") or name == "All tests passed!":
        continue
    # Expanded reporting prefixes group lifecycle hooks with the group path.
    # A requested test whose literal name ends this way still counts.
    if name not in expected and re.fullmatch(r"(?:.* )?\((?:setUpAll|tearDownAll)\)", name):
        continue
    reported.append(name)
if Counter(reported) != expected:
    raise SystemExit(f"Selected test names do not match the reporter: {reported!r}")
PY
fi

# Logs and source probes are the evidence. Don't accumulate a compiled
# Dart kernel for every successful isolated reference runner invocation.
if [[ "$mode" != tooling && ${INCULAR_AUDIT_KEEP_DART_BUILD:-0} != 1 ]]; then
  rm -rf -- "$scratch/build"
fi
