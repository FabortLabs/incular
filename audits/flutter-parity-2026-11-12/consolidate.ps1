$ErrorActionPreference = 'Stop'
$items = @(Get-ChildItem audits/flutter-parity-2026-11-12/findings/*.jsonl | Sort-Object Name | ForEach-Object {
  Get-Content $_.FullName | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json }
}) | Sort-Object severity,package,id
$output = @($items | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
[System.IO.File]::WriteAllLines((Join-Path (Get-Location) 'audits/flutter-parity-2026-11-12/findings.jsonl'), [string[]]$output)
$table = [System.Collections.Generic.List[string]]::new()
$table.Add('## Accepted findings')
$table.Add('')
$table.Add('| ID | Severity | Title | Package | Evidence | Scenario |')
$table.Add('| --- | --- | --- | --- | --- | --- |')
foreach ($item in $items) {
  if ($item.package -in @('paint', 'focus_input', 'semantics') -and $item.id -notin @('paint-001', 'focus_input-001')) { continue }
  $evidence = if ($item.evidence_kind) { $item.evidence_kind } elseif ($item.evidence_type) { $item.evidence_type } else { $item.evidence }
  $scenario = if ($item.scenario) { $item.scenario } else { '—' }
  $table.Add(('| {0} | {1} | {2} | {3} | {4} | {5} |' -f $item.id,$item.severity,$item.title,$item.package,$evidence,$scenario))
}
$table.Add('')
$table.Add('New paint, focus/input and semantics findings are pending independent review and are included in the merged JSONL but excluded from this accepted table until verified.')
$table.Add('')
$table.Add('See individual findings for detailed evidence. The `timeDilation` scenario uses non-equivalent settings because the scheduler option is absent; it illustrates a missing API rather than a same-input timing result. The `AnimatedSize.reverseDuration` observation is API-only, not a demonstrated reverse-timing difference.')
$table.Add('')
$summaryPath = Join-Path (Get-Location) 'audits/flutter-parity-2026-11-12/SUMMARY.md'
$summary = [System.IO.File]::ReadAllText($summaryPath)
$summary = [regex]::Replace($summary, '(?s)## Accepted (?:source )?findings.*?(?=## Harness and existing tests)', (($table -join "`n") + "`n"))
[System.IO.File]::WriteAllText($summaryPath, $summary)
