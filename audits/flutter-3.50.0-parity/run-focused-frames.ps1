[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [ValidateCount(1,5)]
    [string[]]$Scenario
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if (($Scenario | Select-Object -Unique).Count -ne $Scenario.Count) { throw 'Duplicate scenario' }
$catalog = Get-Content (Join-Path $PSScriptRoot 'existing-harness-scenarios.jsonl') |
    ForEach-Object { $_ | ConvertFrom-Json }
$tests = foreach ($case in $Scenario) {
    if ($case -cnotmatch '^[a-z0-9_]+$') { throw "Invalid scenario: $case" }
    $match = @($catalog | Where-Object { $_.registration -eq 'frameScenario' -and $_.description -ceq $case })
    if ($match.Count -ne 1 -or $match[0].description -cne $case -or $match[0].file -cnotmatch '^harness/flutter/test/frames/[a-z0-9_]+_test\.dart$') {
        throw "No unique tracked Flutter frame fixture for $case"
    }
    [pscustomobject]@{Name=$case; File=$match[0].file.Substring('harness/flutter/'.Length)}
}
$ref = Join-Path $root '.tmp/flutter'
$rev = git -C $ref rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $rev -ne 'ab2912035e160d9cad6306df307576da6128ebdb') { throw 'Wrong Flutter reference revision' }
$flutter = Join-Path $ref 'bin/flutter.bat'
if (-not (Test-Path (Join-Path $ref 'bin/cache/dart-sdk/bin/dart.exe'))) { throw 'Pinned SDK is not bootstrapped' }

$scratch = Join-Path $root '.tmp/flutter350-full-audit'
$package = Join-Path $scratch 'frame-reference'
$source = Join-Path $root 'harness/flutter'
New-Item -ItemType Directory -Force $package | Out-Null
Copy-Item "$source/pubspec.yaml","$source/fonts","$source/test" $package -Recurse -Force
$out = Join-Path $scratch ('frame-batches/' + [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmss-fff'))
New-Item -ItemType Directory -Force "$out/frames/flutter" | Out-Null

$env:CI = 'true'
$env:FLUTTER_SUPPRESS_ANALYTICS = 'true'
$env:PARITY_FRAMES_OUT = Join-Path $out 'frames/flutter'
$env:INCULAR_PARITY = '1'
$env:INCULAR_PARITY_DIR = $out
$env:PARITY_STRICT = '1'
$env:PARITY_SCENARIO_EXACT = '1'

Push-Location $package
try {
    & $flutter pub get --offline *> "$out/pub.log"
    if ($LASTEXITCODE -ne 0) { throw "Flutter dependencies failed; see $out/pub.log" }
} finally { Pop-Location }

$results = foreach ($test in $tests) {
    $case = $test.Name
    Push-Location $package
    try {
        & $flutter test --no-pub $test.File --name ('^' + [regex]::Escape($case) + '$') --reporter expanded *> "$out/$case-flutter.log"
        $flutterCode = $LASTEXITCODE
    } finally { Pop-Location }
    if ($flutterCode -ne 0 -or -not (Test-Path "$out/frames/flutter/$case.json")) {
        [pscustomobject]@{Scenario=$case; Status='reference-failed'; Log="$out/$case-flutter.log"}
        continue
    }
    $env:PARITY_SCENARIO = $case
    Push-Location $root
    try {
        cargo test --locked -q -p incular-parity-harness --test frames *> "$out/$case-incular.log"
        $incularCode = $LASTEXITCODE
    } finally { Pop-Location }
    $report = "$out/frames/report-$case.md"
    $line = if (Test-Path $report) { (Select-String -Path $report -Pattern '^## ' | Select-Object -ExpandProperty Line) -join '; ' } else { '' }
    $status = if ($incularCode -eq 0 -and $line -match ' — match \(') { 'match' }
      elseif ($line -match ' — diverg| — mismatch| — structural') { 'mismatch' }
      elseif ($line -and $incularCode -ne 0) { 'strict-failure-review-report' }
      else { 'incular-failed' }
    [pscustomobject]@{Scenario=$case; Status=$status; Report=$report; Detail=$line}
}
$results | ConvertTo-Json -Depth 3 | Set-Content "$out/results.json" -Encoding utf8
$results | Format-List
"Batch output: $out"
if ($results.Status -ne 'match') { exit 1 }
