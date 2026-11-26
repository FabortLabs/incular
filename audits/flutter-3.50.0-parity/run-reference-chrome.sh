#!/usr/bin/env bash
# Run the pinned Flutter BrowserContextMenu originals in a real Chrome host.
set -euo pipefail
if (( $# != 1 )); then
  printf 'Usage: bash run-reference-chrome.sh PATH_TO_CHROME\n' >&2
  exit 2
fi
chrome=$(realpath "$1")
test -x "$chrome"
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo=$(cd -- "$here/../.." && pwd -P)
flutter="$repo/.tmp/flutter"
test "$(git -C "$flutter" rev-parse HEAD)" = ab2912035e160d9cad6306df307576da6128ebdb
scratch=$(mktemp -d "${TMPDIR:-/tmp}/incular-flutter-context-menu.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
mkdir "$scratch/test"
cp -- "$flutter/packages/flutter/test/services/browser_context_menu_test.dart" "$scratch/test/"
cat > "$scratch/pubspec.yaml" <<'YAML'
name: flutter_350_context_menu_probe
publish_to: none
environment:
  sdk: ^3.13.0-0
dependencies:
  flutter:
    sdk: flutter
dev_dependencies:
  flutter_test:
    sdk: flutter
YAML
printf '#!/usr/bin/env bash\nexec %q --headless --no-sandbox --disable-dev-shm-usage "$@"\n' "$chrome" > "$scratch/chrome"
chmod u+x "$scratch/chrome"
export CHROME_EXECUTABLE="$scratch/chrome" CI=true FLUTTER_SUPPRESS_ANALYTICS=true
cd "$scratch"
"$flutter/bin/flutter" pub get --offline
disable='on web disableContextMenu disableContextMenu calls its platform channel method'
enable='on web enableContextMenu enableContextMenu calls its platform channel method'
"$flutter/bin/flutter" test --no-pub --platform chrome --reporter expanded   --name "^($disable|$enable)$" test/browser_context_menu_test.dart | tee result.log
grep -Fq "$disable" result.log
grep -Fq "$enable" result.log
grep -Eq '^[0-9]{2}:[0-9]{2} \+2: All tests passed!' result.log
