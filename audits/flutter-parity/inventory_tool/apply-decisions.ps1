# Applies decisions.json to findings/*.jsonl: overrides first, then the
# default for any remaining pending-review finding. Idempotent. Run from the
# repository root.
$ErrorActionPreference = 'Stop'
$dir = 'audits/flutter-parity'
$d = Get-Content (Join-Path $dir 'decisions.json') -Raw | ConvertFrom-Json
Get-ChildItem (Join-Path $dir 'findings') -Filter *.jsonl | ForEach-Object {
  $path = $_.FullName
  $out = foreach ($line in (Get-Content $path | Where-Object { $_.Trim() })) {
    $o = $line | ConvertFrom-Json
    $ov = $d.overrides.($o.id)
    if ($ov) {
      if ($ov.severity) { $o | Add-Member -Force severity $ov.severity }
      if ($ov.note) { $o | Add-Member -Force verification_note $ov.note }
      if ($ov.status) { $o | Add-Member -Force status $ov.status; $o | Add-Member -Force verified_by 'orchestrator decision (decisions.json)' }
    }
    if ($o.status -eq 'pending-review') {
      $o | Add-Member -Force status $d.default_pending.status
      $o | Add-Member -Force verified_by $d.default_pending.verified_by
    }
    $o | ConvertTo-Json -Compress -Depth 20
  }
  [System.IO.File]::WriteAllLines($path, [string[]]$out)
}
