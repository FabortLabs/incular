#!/usr/bin/env bash
# Compile the real browser keyboard action adapter and exercise it in Chrome.
set -euo pipefail
if (( $# < 1 )); then
  printf 'Usage: bash run.sh PATH_TO_CHROME [CHROME_ARGUMENT ...]\n' >&2
  exit 2
fi
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
root=$(cd -- "$here/../../../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/incular-browser-input-action.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
cd "$root"

cargo build --locked --offline -j1 -p incular-platform --lib --target wasm32-unknown-unknown   --message-format=json > "$scratch/artifacts.json"
mapfile -t artifacts < <(python3 - "$scratch/artifacts.json" <<'PY'
import json
import sys
from pathlib import Path

names = ["incular_platform", "wasm_bindgen", "web_sys", "serde_json"]
libraries = {}
paths = set()
for line in Path(sys.argv[1]).read_text().splitlines():
    item = json.loads(line)
    if item.get("reason") != "compiler-artifact":
        continue
    for filename in map(Path, item["filenames"]):
        if filename.suffix in {".rlib", ".rmeta", ".so", ".dll", ".dylib"}:
            paths.add(filename.parent)
        if filename.suffix == ".rlib" and item["target"]["name"] in names:
            libraries[item["target"]["name"]] = filename
for name in names:
    print(f"{name}={libraries[name]}")
print(*sorted(paths), sep="\n")
PY
)
(( ${#artifacts[@]} >= 5 ))
compiler_args=()
for library in "${artifacts[@]:0:4}"; do compiler_args+=(--extern "$library"); done
for directory in "${artifacts[@]:4}"; do compiler_args+=(-L "dependency=$directory"); done

CARGO_MANIFEST_DIR="$here" CARGO_PKG_NAME=browser_input_action_probe CARGO_PKG_VERSION=0.1.0   rustc --edition 2024 --crate-type cdylib --target wasm32-unknown-unknown     -C debuginfo=0 -C opt-level=1 "${compiler_args[@]}"     "$here/browser_input_action.rs" -o "$scratch/raw.wasm"
"${WASM_BINDGEN:-wasm-bindgen}" --target web --out-name browser_input_action   --out-dir "$scratch" "$scratch/raw.wasm"
cp -- "$here/index.html" "$scratch/"
cp -- "$here/../browser-platform/run.mjs" "$scratch/"
INCULAR_TEST_WASM_MODULE=browser_input_action node "$scratch/run.mjs" "$@"
