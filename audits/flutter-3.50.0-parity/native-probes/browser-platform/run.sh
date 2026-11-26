#!/usr/bin/env bash
# Run the linked Incular wasm module against real Navigator properties in Chrome.
set -euo pipefail
if (( $# < 1 )); then
  printf 'Usage: %s PATH_TO_CHROME [CHROME_ARGUMENT ...]\n' "$0" >&2
  exit 2
fi
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
root=$(cd -- "$here/../../../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/incular-browser-platform.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
cd "$root"

cargo build --locked -j1 -p incular-config --lib --target wasm32-unknown-unknown \
  --message-format=json > "$scratch/artifacts.json"
mapfile -t artifacts < <(python3 - "$scratch/artifacts.json" <<'PY'
import json
import sys
from pathlib import Path

libs = {}
directories = set()
for line in Path(sys.argv[1]).read_text().splitlines():
    record = json.loads(line)
    if record.get("reason") != "compiler-artifact":
        continue
    target = record["target"]["name"]
    for filename in map(Path, record["filenames"]):
        if filename.suffix in {".rlib", ".rmeta", ".so", ".dylib", ".dll"}:
            directories.add(filename.parent)
        if target in {"incular_config", "wasm_bindgen"} and filename.suffix == ".rlib":
            libs[target] = filename
print(libs["incular_config"])
print(libs["wasm_bindgen"])
print(*sorted(directories), sep="\n")
PY
)
(( ${#artifacts[@]} >= 3 ))
search=()
for directory in "${artifacts[@]:2}"; do
  search+=(-L "dependency=$directory")
done

CARGO_MANIFEST_DIR="$here" CARGO_PKG_NAME=browser_platform_probe CARGO_PKG_VERSION=0.1.0 \
  rustc --edition 2024 --crate-type cdylib --target wasm32-unknown-unknown \
    -C debuginfo=0 -C opt-level=1 --extern "incular_config=${artifacts[0]}" \
    --extern "wasm_bindgen=${artifacts[1]}" "${search[@]}" \
    "$here/browser_platform.rs" -o "$scratch/raw_browser_platform.wasm"
"${WASM_BINDGEN:-wasm-bindgen}" --target web --out-name browser_platform \
  --out-dir "$scratch" "$scratch/raw_browser_platform.wasm"
cp -- "$here/index.html" "$here/run.mjs" "$scratch/"
node "$scratch/run.mjs" "$@"
