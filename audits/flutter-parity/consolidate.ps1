# Merges findings/*.jsonl into findings.jsonl (sorted by severity, package, id)
# and regenerates the findings tables in SUMMARY.md between the markers
# <!-- findings:begin --> and <!-- findings:end -->. Run from the repo root.
$ErrorActionPreference = 'Stop'
$dir = 'audits/flutter-parity'
$items = @(Get-ChildItem (Join-Path $dir 'findings') -Filter '*.jsonl' | Sort-Object Name | ForEach-Object {
  $file = $_.Name
  $n = 0
  Get-Content $_.FullName | ForEach-Object {
    $n++
    if ($_.Trim()) {
      try { $_ | ConvertFrom-Json } catch { throw "${file}:${n}: invalid JSON" }
    }
  }
}) | Sort-Object severity, package, id
$dupes = $items | Group-Object id | Where-Object Count -gt 1
if ($dupes) { throw "duplicate finding ids: $($dupes.Name -join ', ')" }
$output = @($items | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 20 })
[System.IO.File]::WriteAllLines((Join-Path (Resolve-Path $dir) 'findings.jsonl'), [string[]]$output)

function Cell($v) { if ($null -eq $v -or "$v" -eq '') { '—' } else { ("$v" -replace '\|', '\|' -replace "`r?`n", ' ') } }
function Table($rows, $title) {
  $t = [System.Collections.Generic.List[string]]::new()
  $t.Add("### $title ($(@($rows).Count))")
  $t.Add('')
  if (!@($rows).Count) { $t.Add('None.'); $t.Add(''); return $t }
  $t.Add('| ID | Sev | Impact | Title | Evidence | Scenario |')
  $t.Add('| --- | --- | --- | --- | --- | --- |')
  foreach ($i in $rows) {
    $t.Add(('| [{0}](findings/{1}.md) | {2} | {3} | {4} | {5} | {6} |' -f $i.id, $i.package, $i.severity, (Cell $i.impact_class), (Cell $i.title), (Cell $i.evidence_kind), (Cell $i.scenario)))
  }
  $t.Add('')
  return $t
}
$block = [System.Collections.Generic.List[string]]::new()
$block.Add('<!-- findings:begin -->')
$block.AddRange([string[]](Table @($items | Where-Object status -eq 'accepted') 'Accepted (independently verified)'))
$block.AddRange([string[]](Table @($items | Where-Object { $_.status -notin @('accepted', 'rejected') }) 'Pending verification (not accepted)'))
$block.AddRange([string[]](Table @($items | Where-Object status -eq 'rejected') 'Rejected or no longer reproducing'))
$block.Add('<!-- findings:end -->')
$summaryPath = Join-Path (Resolve-Path $dir) 'SUMMARY.md'
$summary = [System.IO.File]::ReadAllText($summaryPath)
if ($summary -notmatch '<!-- findings:begin -->') { throw 'SUMMARY.md lacks findings markers' }
$summary = [regex]::Replace($summary, '(?s)<!-- findings:begin -->.*?<!-- findings:end -->', [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $block -join "`n" })
[System.IO.File]::WriteAllText($summaryPath, $summary)
$items | Group-Object status | ForEach-Object { "{0}: {1}" -f $_.Name, $_.Count }
