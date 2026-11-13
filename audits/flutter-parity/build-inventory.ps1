$ErrorActionPreference = 'Stop'
$flutter = 'C:/Manual Installs/flutter/packages/flutter/lib/src'
$rust = @{}
Get-ChildItem crates -Recurse -Filter '*.rs' | ForEach-Object {
    $path = $_.FullName.Substring((Get-Location).Path.Length + 1).Replace('\','/')
    $n = 0
    foreach ($line in [System.IO.File]::ReadLines($_.FullName)) {
        $n++
        if ($line -match '^\s*pub(?:\([^)]*\))?\s+(?:(?:unsafe|async)\s+)?(?:struct|enum|trait|type|fn|const|static)\s+([A-Za-z][A-Za-z0-9_]*)') {
            $name = $Matches[1]
            if (!$rust.ContainsKey($name)) { $rust[$name] = "${path}:$n" }
        }
    }
}
$rows = [System.Collections.Generic.List[string]]::new()
$rows.Add('# Flutter parity inventory (preliminary declaration index)')
$rows.Add('')
$rows.Add('This mechanically generated index is incomplete: top-level functions, constants, class members, conditional exports, and widget-dependent rendering, painting, gestures and services require manual review. A name match is only a candidate counterpart, not proof of equivalent behavior. No entry is checked solely by being listed here. Each unreviewed entry is marked not checked because this audit is incomplete; package assignments are preliminary.')
$rows.Add('')
$rows.Add('| Flutter declaration | Candidate Incular declaration | Package | Coverage |')
$rows.Add('| --- | --- | --- | --- |')
$dirs = @('animation','physics','scheduler','widgets')
foreach ($dir in $dirs) {
  Get-ChildItem "$flutter/$dir" -Recurse -Filter '*.dart' | Sort-Object FullName | ForEach-Object {
    $file = $_
    $relative = $file.FullName.Substring($flutter.Length + 1).Replace('\','/')
    $pkg = switch -Regex ($relative) {
      '^physics/' { 'physics'; break }
      '^scheduler/' { 'anim_core'; break }
      '^animation/(curves|tween|tween_sequence)' { 'anim_curves'; break }
      '^animation/animations' { 'anim_compose'; break }
      '^animation/' { 'anim_core'; break }
      'implicit_animations' { 'implicit'; break }
      'transitions' { 'transitions'; break }
      'animated_(switcher|cross_fade|size|scroll_view|list|grid)' { 'switchers'; break }
      'navigator|routes|hero|overlay|router|pop_scope' { 'navigation'; break }
      'sliver|viewport' { 'slivers'; break }
      'scroll|page_view|list_wheel|overscroll|scrollbar' { 'scroll'; break }
      'gesture|drag|dismissible|interactive_viewer|reorderable' { 'gestures'; break }
      'focus|actions|shortcuts|keyboard|mouse|pointer' { 'focus_input'; break }
      'semantics' { 'semantics'; break }
      'text|editable|selection|autofill' { 'text'; break }
      'basic|container|layout_builder|table|visibility' { 'layout'; break }
      'image|decorated|clip|paint' { 'paint'; break }
      default { 'misc' }
    }
    $n = 0
    $inCurves = $false
    foreach ($line in [System.IO.File]::ReadLines($file.FullName)) {
      $n++
      if ($relative -eq 'animation/curves.dart' -and $line -match '^abstract (?:final )?class Curves\b|^class Curves\b') { $inCurves = $true }
      if ($inCurves -and $line -eq '}') { $inCurves = $false }
      if ($line -match '^(?:(?:abstract|base|final|sealed|interface)\s+)*(?:class|mixin(?:\s+class)?|enum|extension\s+type)\s+([A-Za-z][A-Za-z0-9_]*)\b') {
        $name = $Matches[1]
        $candidate = if ($rust.ContainsKey($name)) { $rust[$name] } else { 'UNRESOLVED' }
        $coverage = 'not checked'
        $itemPkg = $pkg
        switch ($name) {
          'RichText' { $coverage = 'reading (partial: span styles, text-001)'; $itemPkg = 'text' }
          'SizedBox' { $coverage = 'reading (partial: hit testing, focus_input-001)'; $itemPkg = 'focus_input' }
          'ModalRoute' { $coverage = 'reading (partial: lookup, navigation-001)'; $itemPkg = 'navigation' }
          'BackdropFilter' { $coverage = 'reading (partial: filter kinds, paint-001)'; $itemPkg = 'paint' }
          'Expansible' { $coverage = 'reading (partial: expansion animation, misc-001)'; $itemPkg = 'misc' }
        }
        $rows.Add("| ``${relative}:$n`` $name | $candidate | $itemPkg | $coverage |")
      } elseif ($line -match '^(?:const|final)\s+(?:[\w<>?,.]+\s+)?([A-Za-z][A-Za-z0-9_]*)\s*=') {
        $name = $Matches[1]
        $candidate = if ($rust.ContainsKey($name)) { $rust[$name] } else { 'UNRESOLVED' }
        $rows.Add("| ``${relative}:$n`` $name (constant) | $candidate | $pkg | not checked |")
      } elseif ($line -match '^(?:[\w<>?,.]+\s+)+([A-Za-z][A-Za-z0-9_]*)\s*(?:<[^>]+>)?\(') {
        $name = $Matches[1]
        $candidate = if ($rust.ContainsKey($name)) { $rust[$name] } else { 'UNRESOLVED' }
        $rows.Add("| ``${relative}:$n`` $name (function) | $candidate | $pkg | not checked |")
      } elseif ($inCurves -and $line -match '^  static const\s+\w+\s+([A-Za-z][A-Za-z0-9_]*)\s*=') {
        $name = $Matches[1]
        $rows.Add("| ``${relative}:$n`` Curves.$name (constant) | UNRESOLVED | anim_curves | not checked |")
      }
    }
  }
}
[System.IO.File]::WriteAllLines((Join-Path (Get-Location) 'audits/flutter-parity/inventory.md'), $rows)
