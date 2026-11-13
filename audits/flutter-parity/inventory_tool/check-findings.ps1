# Lists findings whose JSONL line lacks a key of the shared schema
# (WORKER-BRIEF.md). Run from the repository root.
$ErrorActionPreference = 'Stop'
$required = 'id','package','severity','category','impact_class','title','flutter_ref','incular_ref','evidence_kind','scenario','first_divergence','evidence','impact','cause','fix_direction','confidence','limitations','status'
Get-ChildItem audits/flutter-parity/findings -Filter *.jsonl | ForEach-Object {
  $file = $_.Name
  Get-Content $_.FullName | Where-Object { $_.Trim() } | ForEach-Object {
    $o = $_ | ConvertFrom-Json
    $names = $o.PSObject.Properties.Name
    $missing = @($required | Where-Object { $_ -notin $names })
    if ($missing) { '{0} {1}: missing {2}' -f $file, $o.id, ($missing -join ',') }
  }
}
