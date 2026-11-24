[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [ValidatePattern('^(animation|foundation|gestures|painting|physics|rendering|scheduler|semantics|services|widget_previews|widgets)/(?:[a-z0-9_]+/)*[a-z0-9_]+_test\.dart$')]
    [string]$Test,
    [Parameter(Mandatory=$true)]
    [ValidateCount(1,5)]
    [string[]]$FullName
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
if (@($FullName | Where-Object { [string]::IsNullOrWhiteSpace($_) }).Count -ne 0) {
    throw 'Give exact full test names, including group prefixes'
}
$original = Join-Path $reference "packages/flutter/test/$Test"
if (-not (Test-Path $original -PathType Leaf)) { throw "Unknown pinned upstream fixture: $Test" }
$pattern = '^(?:' + (($FullName | ForEach-Object { [regex]::Escape($_) }) -join '|') + ')$'
$timestamp = [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$scratch = Join-Path $root ".tmp/flutter350-full-audit/upstream-runs/$timestamp"
New-Item -ItemType Directory -Force $scratch | Out-Null
$flutterGoldens = (Join-Path $reference 'packages/flutter_goldens').Replace('\','/').Replace("'","''")
@"
name: incular_upstream_reference_probe
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
    path: '$flutterGoldens'
"@ | Set-Content "$scratch/pubspec.yaml" -Encoding utf8

$previousFlutterRoot = $env:FLUTTER_ROOT
$env:CI = 'true'
$env:FLUTTER_SUPPRESS_ANALYTICS = 'true'
Push-Location $scratch
try {
    & $flutter pub get --offline *> pub.log
    if ($LASTEXITCODE -ne 0) { throw "Offline Flutter dependencies failed; see $scratch/pub.log" }
    Write-Output "Upstream run: $scratch"
    # Invoke the pinned tool directly so cmd.exe cannot strip the ^ anchor
    # from a single short name or interpret regex characters as shell syntax.
    $env:FLUTTER_ROOT = $reference
    $dart = Join-Path $reference 'bin/cache/dart-sdk/bin/dart.exe'
    $tool = Join-Path $reference 'bin/cache/flutter_tools.snapshot'
    $toolPackages = Join-Path $reference 'packages/flutter_tools/.dart_tool/package_config.json'
    & $dart "--packages=$toolPackages" $tool test --no-pub $original --name $pattern --reporter expanded 2>&1 |
        Tee-Object -FilePath 'result.log'
    $testExitCode = $LASTEXITCODE
    if ($testExitCode -ne 0) { exit $testExitCode }
    $reported = @(Get-Content 'result.log' | ForEach-Object {
        if ($_ -match '^\d+:\d+ \+\d+: (?<testName>.+)$') {
            $name = $Matches.testName
            if (-not $name.StartsWith('loading ') -and $name -ne 'All tests passed!') { $name }
        }
    })
    if ($reported.Count -ne $FullName.Count -or @($FullName | Where-Object { $_ -cnotin $reported }).Count) {
        throw "Selected test names do not match the expanded reporter: $($reported -join '; ')"
    }
} finally {
    $env:FLUTTER_ROOT = $previousFlutterRoot
    Pop-Location
}
