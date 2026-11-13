# One-time migration: split the hand-corrected inventory.md into per-package
# coverage files (coverage/<package>.jsonl) that workers own and edit. Each
# line is one inventory item. render-inventory.ps1 rebuilds inventory.md.
# Refuses to overwrite existing coverage files.
$ErrorActionPreference = 'Stop'
$dir = 'audits/flutter-parity'
$out = Join-Path $dir 'coverage'
if (Test-Path $out) { throw "coverage/ already exists; not overwriting" }
New-Item -ItemType Directory $out | Out-Null
$rows = @{}
foreach ($line in [System.IO.File]::ReadLines((Join-Path $dir 'inventory.md'))) {
  if ($line -notmatch '^\| `([^`]+):(\d+)` (.+?) \| (.*?) \| (\w+) \| (.*) \|$') { continue }
  $file = $Matches[1]; $ln = [int]$Matches[2]; $name = $Matches[3]
  $cand = $Matches[4]; $pkg = $Matches[5]; $cov = $Matches[6]
  $status = if ($cov -match '^not checked') { 'unchecked' } elseif ($cov -match 'scenario') { 'partial-scenario' } else { 'partial-reading' }
  $obj = [ordered]@{
    file = $file; line = $ln; name = $name; package = $pkg
    counterpart = $cand; scope = 'in'; status = $status; coverage = $cov
    findings = @(); scenarios = @()
  }
  if (!$rows.ContainsKey($pkg)) { $rows[$pkg] = [System.Collections.Generic.List[string]]::new() }
  $rows[$pkg].Add(($obj | ConvertTo-Json -Compress -Depth 5))
}
foreach ($pkg in $rows.Keys) {
  [System.IO.File]::WriteAllLines((Join-Path (Resolve-Path $out) "$pkg.jsonl"), [string[]]$rows[$pkg])
}
$rows.Keys | ForEach-Object { "{0}: {1}" -f $_, $rows[$_].Count }
