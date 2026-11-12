$ErrorActionPreference = 'Stop'
$root = (Get-Location).Path
$items = @(Get-Content 'audits/flutter-parity-2026-11-12/declarations.jsonl' | ForEach-Object { $_ | ConvertFrom-Json })
$symbols = @{}
Get-ChildItem crates -Recurse -Filter '*.rs' | ForEach-Object {
  $path = $_.FullName.Substring($root.Length + 1).Replace('\','/')
  $n = 0
  foreach ($line in [System.IO.File]::ReadLines($_.FullName)) {
    $n++
    if ($line -match '^\s*pub\s+(?:(?:unsafe|async|const)\s+)?(?:struct|enum|trait|type|fn|const|static)\s+([A-Za-z][A-Za-z0-9_]*)') {
      if (!$symbols.ContainsKey($Matches[1])) { $symbols[$Matches[1]] = "${path}:$n" }
    }
  }
}
function Package($file, $name) {
  $s = "$file $name"
  if ($file -match '^animation/curves.dart|^animation/tween') { return 'anim_curves' }
  if ($file -match '^animation/animations.dart|^animation/animation_style.dart') { return 'anim_compose' }
  if ($file -match '^(animation|scheduler)/') { return 'anim_core' }
  if ($file -match '^physics/|^widgets/scroll_simulation.dart') { return 'physics' }
  if ($file -match '^gestures/' -or $s -match '(Gesture|Drag|Dismissible|InteractiveViewer|Reorderable)') { return 'gestures' }
  if ($s -match '(ImplicitlyAnimated|AnimatedContainer|AnimatedPadding|AnimatedAlign|AnimatedPositioned|AnimatedOpacity|AnimatedDefaultTextStyle|AnimatedPhysicalModel|AnimatedRotation|AnimatedScale|AnimatedSlide|AnimatedFractionallySizedBox|TweenAnimationBuilder)') { return 'implicit' }
  if ($s -match '(AnimatedSwitcher|AnimatedCrossFade|AnimatedSize|AnimatedList|AnimatedGrid|AnimatedModalBarrier|SliverAnimated)') { return 'switchers' }
  if ($s -match '(Transition|AnimatedBuilder|AnimatedWidget|ListenableBuilder)') { return 'transitions' }
  if ($s -match '(Navigator|ModalRoute|PageRoute|Overlay|Hero|Router|PopScope|RouteObserver|RouteAware)') { return 'navigation' }
  if ($s -match '(Scroll|PageView|PageController|ViewportOffset|Overscroll|Scrollbar|NestedScroll|ListWheel)') { return 'scroll' }
  if ($s -match '(Sliver|Viewport|RenderViewport)') { return 'slivers' }
  if ($s -match '(Semantics|SemanticsNode)') { return 'semantics' }
  if ($s -match '(Focus|Shortcut|Action|Intent|KeyEvent|Keyboard|MouseRegion|Pointer|HitTest|Listener|IgnorePointer|AbsorbPointer)') { return 'focus_input' }
  if ($s -match '(Text|Font|Caret|Selection|Editable|Autofill|Paragraph|InlineSpan|TextInput)') { return 'text' }
  if ($file -match '^painting/' -or $s -match '(Paint|Clip|Decoration|Decorated|Filter|Shader|Opacity|Image|PhysicalModel|Repaint|ColorFilter|Backdrop)') { return 'paint' }
  if ($s -match '(Flex|Row|Column|Stack|Wrap|Flow|Layout|Align|Center|Constrained|OverflowBox|SizedBox|AspectRatio|FittedBox|Intrinsic|Baseline|Table|Padding|Transform|Offstage|Visibility|Positioned)') { return 'layout' }
  return 'misc'
}
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add('# Flutter parity inventory — candidate declarations')
$lines.Add('')
$lines.Add('AST-extracted public declarations from Flutter 3.41.7. This index is not yet a completed inventory: entries from rendering, painting, gestures and services require widget-dependency relevance review, and package assignments and counterpart name matches need manual verification. UNRESOLVED means not yet searched, not a confirmed missing API. Every not-checked entry is pending the audit; partial coverage notes do not assert full behavioral review. Getter and setter declarations of one name in a file are grouped as one item.')
$lines.Add('')
$lines.Add('| Flutter file:line and item | Candidate Incular counterpart | Package | Scope / coverage |')
$lines.Add('| --- | --- | --- | --- |')
foreach ($group in ($items | Group-Object { "$($_.file)|$($_.name)" })) {
  $first = $group.Group[0]
  $name = $first.name
  $candidate = if ($symbols.ContainsKey($name)) { $symbols[$name] } else { 'UNRESOLVED' }
  $package = Package $first.file $name
  $coverage = if ($first.scope -eq 'primary') { 'not checked: pending audit' } else { 'not checked: widget relevance and behavior pending' }
  if ($first.file -match '^animation/(curves|tween|tween_sequence)\.dart$') {
    $coverage = 'partial scenarios and reading: anim_curves; see package coverage and open cases'
  }
  if ($first.file -match '^physics/' -or $name -in @('BouncingScrollSimulation','ClampingScrollSimulation')) {
    $coverage = 'partial scenarios and reading: physics; see package coverage and open cases'
  }
  if ($first.file -eq 'widgets/implicit_animations.dart') {
    $coverage = 'partial scenarios and reading: implicit; see package coverage and open cases'
  }
  if ($first.file -eq 'widgets/transitions.dart') {
    $coverage = 'partial scenarios and reading: transitions; see package coverage and open cases'
  }
  switch ($name) {
    'AnimationController' { $coverage = 'partial scenarios and reading: anim_core'; $package = 'anim_core' }
    'AnimationBehavior' { $coverage = 'partial reading: anim_core'; $package = 'anim_core' }
    'Ticker' { $coverage = 'partial scenarios and reading: anim_core'; $package = 'anim_core' }
    'TickerFuture' { $coverage = 'partial scenarios and reading: anim_core'; $package = 'anim_core' }
    'TickerMode' { $coverage = 'partial reading: anim_core-005'; $package = 'anim_core' }
    'timeDilation' { $coverage = 'partial diagnostic and reading: anim_core-003'; $package = 'anim_core' }
    'TrainHoppingAnimation' { $coverage = 'partial scenario and reading: anim_compose-001'; $package = 'anim_compose' }
    'ScrollController' { $coverage = 'partial scenarios and reading: scroll-001, scroll-003'; $package = 'scroll' }
    'PageController' { $coverage = 'partial scenario and reading: scroll-002'; $package = 'scroll' }
    'PageView' { $coverage = 'partial scenario and reading: scroll'; $package = 'scroll' }
    'Scrollable' { $coverage = 'partial reading: scroll-004'; $package = 'scroll' }
    'SliverVariedExtentList' { $coverage = 'scenario and reading: slivers-001'; $package = 'slivers' }
    'GestureDetector' { $coverage = 'partial scenarios and reading: gestures'; $package = 'gestures' }
    'AnimatedSwitcher' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'AnimatedCrossFade' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'AnimatedSize' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'AnimatedList' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'AnimatedGrid' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'SliverAnimatedList' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'SliverAnimatedGrid' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'AnimatedModalBarrier' { $coverage = 'partial scenarios and reading: switchers'; $package = 'switchers' }
    'RichText' { $coverage = 'partial reading: text-001'; $package = 'text' }
    'SizedBox' { $coverage = 'partial reading: focus_input-001'; $package = 'focus_input' }
    'ModalRoute' { $coverage = 'partial reading: navigation-001'; $package = 'navigation' }
    'BackdropFilter' { $coverage = 'partial reading: paint-001'; $package = 'paint' }
    'Expansible' { $coverage = 'partial reading: misc-001'; $package = 'misc' }
  }
  $locations = ($group.Group | ForEach-Object { $_.line }) -join ','
  $lines.Add("| ``$($first.file):$locations`` $name | $candidate | $package | $coverage |")
}
[System.IO.File]::WriteAllLines((Join-Path $root 'audits/flutter-parity-2026-11-12/inventory.md'), $lines)
