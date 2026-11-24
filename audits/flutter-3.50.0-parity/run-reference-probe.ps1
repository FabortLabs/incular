[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [ValidatePattern('^[a-z0-9_]+_test\.dart$')]
    [string]$Test
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$reference = Join-Path $root '.tmp/flutter'
$revision = git -C $reference rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $revision -ne 'ab2912035e160d9cad6306df307576da6128ebdb') {
    throw 'Expected the already-bootstrapped Flutter 3.50.0 audit checkout'
}
$flutter = Join-Path $reference 'bin/flutter.bat'
if (-not (Test-Path (Join-Path $reference 'bin/cache/dart-sdk/bin/dart.exe'))) {
    throw 'Pinned SDK is not bootstrapped'
}
$package = Join-Path $PSScriptRoot 'reference-probes'
$fixture = Join-Path $package "test/$Test"
if (-not (Test-Path $fixture -PathType Leaf)) { throw "Unknown pinned reference fixture: $Test" }
$timestamp = [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$scratch = Join-Path $root ".tmp/flutter350-full-audit/reference-runs/$timestamp"
New-Item -ItemType Directory -Force "$scratch/test" | Out-Null
Copy-Item "$package/pubspec.yaml" "$scratch/pubspec.yaml"
Copy-Item $fixture "$scratch/test/$Test"

$env:CI = 'true'
$env:FLUTTER_SUPPRESS_ANALYTICS = 'true'
Push-Location $scratch
try {
    & $flutter pub get --offline *> pub.log
    if ($LASTEXITCODE -ne 0) { throw "Offline Flutter dependencies failed; see $scratch/pub.log" }
    & $flutter test --no-pub "test/$Test" --reporter expanded 2>&1 | Tee-Object -FilePath 'result.log'
    exit $LASTEXITCODE
} finally { Pop-Location }
