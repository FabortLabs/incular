# Paired Flutter 3.50.0 comparisons

Six focused expected-behavior assertions passed on Flutter and failed on
Incular. These are six distinct cases, not the entire upstream suite.
Incular framework source is unchanged from `c647cb85`. Flutter reported
3.50.0, framework `ab2912035e`, engine `371f77202a`, Dart 3.14.0.

| ID | Scenario | Flutter 3.50 run | Incular run |
| --- | --- | --- | --- |
| C01 | Loose max 100 px, Row Min, two equal Expanded children | Width 100, child 2 x=50 | Width 0, child 2 x=0 |
| C02 | Default Table, loose max 100, two children each naturally 80 px | Widths 50/50, second child x=50 | Widths 80/80, second child x=80 |
| C03 | MergeSemantics with two labelled, actionable descendants | Advertises tap and calls North only | Merges labels, but the merged node does not advertise activation |
| C04 | Start ticker in an already-draining transient frame batch | No current tick; next tick elapsed 16 ms | Current batch already emits 0 ns |
| C05 | Wrap children 10.1+10.3 at available width 20.4 | Second child y=0 | Second child y=10 |
| C06 | ScrollMetrics at pixels=-20, extents 0..200, viewport 100 | atEdge=false; extentInside=80 | atEdge=true; extentInside=100 |

The independent test for C06 supplies DPR 2 on both constructed snapshots;
it does not exercise the Incular controller's separate hardcoded DPR. For
C03 the Incular failure occurred before dispatch, so no runtime claim about
post-advertisement dispatch follows. No native accessibility or GPU pixel
verification is implied. Test setup failures while constructing the temporary
fixtures were corrected before the runs above.

## Running the comparators

The scratch directory on the audit laptop is `.tmp/flutter350-full-audit/`.
The Flutter fixture package used only the Flutter SDK and flutter_test; run:

```powershell
$flutter = (Resolve-Path .tmp/flutter/bin/flutter.bat).Path
Push-Location .tmp/flutter350-full-audit/flutter-probes
& $flutter test --no-pub test/contract_test.dart --reporter expanded
Pop-Location
```

The four Rust files below were copied to temporary paths in the named crate's
`tests/` directory for only the corresponding command. Check that a path
does not exist before copying and remove only the temporary copy afterwards.
Keep the asserts (they are intentionally red on the audited Incular source).
For subsequent runs, prefer the permanent targets in the
[backfill ledger](test-backfill.tsv). These assertions are intentionally
red until their underlying contracts are fixed. No workspace-wide or CI
run is part of this audit.

| Scratch source | Temporary target and crate |
| --- | --- |
| `layout-probes.rs` | `cargo test --locked -p incular-widgets --test audit_f350_layout_tmp flutter350_` |
| `boundary-probes.rs` | `cargo test --locked -p incular-widgets --test audit_f350_boundary_tmp flutter350_` |
| `semantics-probe.rs` | `cargo test --locked -p incular-runtime --test audit_f350_semantics_tmp flutter350_` |
| `ticker-probe.rs` | `cargo test --locked -p incular-runtime --test audit_f350_ticker_tmp flutter350_` |

Logs are in the scratch directory (`flutter-contract.log`, `layout.log`,
`boundaries.log`, `semantics.log`, `ticker.log`). The deterministic fixtures
are transcribed below so the documentation does not depend on scratch files.

### Flutter neutral fixtures

```dart
import 'dart:ui' as ui;
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget loose100(Widget child) => Directionality(textDirection: TextDirection.ltr,
  child: Center(child: SizedBox(width: 100, height: 30,
    child: Align(alignment: Alignment.topLeft, child: child))));

void main() {
  testWidgets('min Row retains the flexible allocations', (tester) async {
    const row = ValueKey('row'), left = ValueKey('left'), right = ValueKey('right');
    await tester.pumpWidget(loose100(const Row(key: row, mainAxisSize: MainAxisSize.min,
      children: [
        Expanded(child: SizedBox(key: left, width: 10, height: 10)),
        Expanded(child: SizedBox(key: right, width: 10, height: 10)),
      ])));
    expect(tester.getSize(find.byKey(row)).width, 100);
    expect(tester.getTopLeft(find.byKey(right)).dx - tester.getTopLeft(find.byKey(left)).dx, 50);
  });

  testWidgets('default Table allocates flex before measuring children', (tester) async {
    const left = ValueKey('left'), right = ValueKey('right');
    await tester.pumpWidget(loose100(Table(children: const [
      TableRow(children: [SizedBox(key: left, width: 80, height: 10),
        SizedBox(key: right, width: 80, height: 10)]),
    ])));
    expect(tester.getSize(find.byKey(left)).width, 50);
    expect(tester.getSize(find.byKey(right)).width, 50);
    expect(tester.getTopLeft(find.byKey(right)).dx - tester.getTopLeft(find.byKey(left)).dx, 50);
  });

  testWidgets('MergeSemantics advertises and routes the first tap', (tester) async {
    final handle = tester.ensureSemantics();
    final seen = <String>[];
    try {
    await tester.pumpWidget(Directionality(textDirection: TextDirection.ltr,
      child: MergeSemantics(child: Column(children: [
        Semantics(label: 'North', onTap: () => seen.add('North'),
          child: const SizedBox(width: 60, height: 40)),
        Semantics(label: 'South', onTap: () => seen.add('South'),
          child: const SizedBox(width: 60, height: 40)),
      ]))));
    final node = tester.getSemantics(find.byType(MergeSemantics));
    expect(node.getSemanticsData().hasAction(ui.SemanticsAction.tap), isTrue);
    tester.binding.pipelineOwner.semanticsOwner!.performAction(node.id, ui.SemanticsAction.tap);
    expect(seen, ['North']);
    } finally { handle.dispose(); }
  });

  testWidgets('Ticker started in transient waits until the next batch', (tester) async {
    final ticks = <Duration>[];
    final ticker = Ticker(ticks.add);
    addTearDown(ticker.dispose);
    tester.binding.scheduleFrameCallback((_) { ticker.start(); });
    await tester.pump();
    expect(ticks, isEmpty);
    await tester.pump(const Duration(milliseconds: 16));
    expect(ticks, [const Duration(milliseconds: 16)]);
    ticker.stop();
  });
  testWidgets('Wrap keeps an exact fractional edge on the same run', (tester) async {
    const left = ValueKey('left'), right = ValueKey('right');
    await tester.pumpWidget(loose100(ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 20.4),
      child: const Wrap(children: [
        SizedBox(key: left, width: 10.1, height: 10),
        SizedBox(key: right, width: 10.3, height: 10),
      ]),
    )));
    expect(tester.getTopLeft(find.byKey(right)).dy - tester.getTopLeft(find.byKey(left)).dy, 0);
  });
  test('ScrollMetrics distinguishes overscroll from the exact edge', () {
    final m = FixedScrollMetrics(minScrollExtent: 0, maxScrollExtent: 200,
      pixels: -20, viewportDimension: 100, axisDirection: AxisDirection.down, devicePixelRatio: 2);
    expect(m.atEdge, isFalse);
    expect(m.extentInside, 80);
  });
}
```

### Incular fixtures

Append the following to copies of `incular-widgets/tests/wrap_table_layout.rs`,
using separate files for layout and boundary probes as indicated above:

```rust
#[test]
fn flutter350_min_row_retains_bounded_expanded_allocations() {
    use incular_config::MainAxisSize;
    use incular_widgets::{Expanded, Row};
    let mut tree = WidgetTree::new();
    let root = tree.mount(
        Row::new([Expanded::new(box_(10., 10.)), Expanded::new(box_(10., 10.))])
            .main_axis_size(MainAxisSize::Min)
            .text_direction(TextDirection::Ltr)
            .into(),
    ).unwrap();
    tree.layout(Constraints::loose(Size::new(100., 30.))).unwrap();
    let kids = children_of(&tree, root);
    assert_eq!((bounds(&tree, root).2, bounds(&tree, kids[1]).0), (100., 50.));
}
#[test]
fn flutter350_default_table_flex_ignores_natural_cell_width() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(Table::new(2, [box_(80., 10.), box_(80., 10.)]).into()).unwrap();
    tree.layout(Constraints::loose(Size::new(100., 30.))).unwrap();
    let kids = children_of(&tree, root);
    assert_eq!(
        (bounds(&tree, kids[0]).2, bounds(&tree, kids[1]).0, bounds(&tree, kids[1]).2),
        (50., 50., 50.),
    );
}

#[test]
fn flutter350_wrap_keeps_exact_fractional_edge() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(Wrap::new([box_(10.1,10.),box_(10.3,10.)])
        .text_direction(TextDirection::Ltr).into()).unwrap();
    tree.layout(Constraints::loose(Size::new(20.4,40.))).unwrap();
    let kids=children_of(&tree,root);
    assert_eq!(bounds(&tree,kids[1]).1,0.);
}
#[test]
fn flutter350_scroll_metrics_distinguish_overscroll_from_edge() {
    let m=incular_scroll::ScrollMetrics { min_scroll_extent:0., max_scroll_extent:200.,
        pixels:-20., viewport_dimension:100., device_pixel_ratio:2., ..Default::default() };
    assert_eq!((m.at_edge(),m.extent_inside()), (false,80.));
}
```

C03 complete integration-test source:

```rust
use std::{cell::Cell, rc::Rc};
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind};
use incular_widgets::{Column, MergeSemantics, Semantics, SizedBox, Widget};

#[test]
fn flutter350_merge_semantics_dispatches_first_descendant_activate() {
    let north_calls = Rc::new(Cell::new(0));
    let south_calls = Rc::new(Cell::new(0));
    let child = |name, acc: Rc<Cell<u32>>| -> Widget {
        Semantics::new(SizedBox::new().width(60.).height(40.)).label(name)
            .on_tap(move || acc.set(acc.get()+1)).into()
    };
    let first = child("North", north_calls.clone());
    let second = child("South", south_calls.clone());
    let mut rt=Runtime::new(Widget::from(MergeSemantics::new(Column::new([first, second])))).unwrap();
    rt.run_frame(Constraints::tight(Size::new(240.,160.))).unwrap();
    let (id, advertised)=rt.tree().semantics().iter().find_map(|(id,n)|
        (n.label.as_deref()==Some("North\nSouth")).then_some((id,n.actions.contains(&SemanticActionKind::Activate)))).unwrap();
    assert!(advertised, "the merged node must advertise its first reachable tap action");
    assert!(rt.dispatch_semantic_action(id,SemanticAction::Activate));
    assert_eq!((north_calls.get(),south_calls.get()),(1,0));
}
```

C04 append to a copy of `incular-runtime/tests/frame_scheduler.rs`:

```rust
#[test]
fn flutter350_ticker_started_in_transient_waits_for_next_batch() {
    let mut runtime = runtime();
    let ticks = Rc::new(RefCell::new(Vec::new()));
    let ticker = Rc::new(Ticker::new({
        let ticks = ticks.clone();
        move |elapsed| ticks.borrow_mut().push(elapsed)
    }));
    schedule_frame_callback({
        let ticker = ticker.clone();
        move |_| { ticker.start(); }
    });
    let base = Instant::now();
    runtime.run_frame_at(bounds(), base).unwrap();
    let first_batch = ticks.borrow().clone();
    runtime.run_frame_at(bounds(), base + Duration::from_millis(16)).unwrap();
    assert!(first_batch.is_empty(), "new ticker ran in the current batch: {first_batch:?}");
    assert_eq!(*ticks.borrow(), [Duration::from_millis(16)]);
}
```

### C04 retained recheck

Pinned Flutter and Incular both defer a ticker started from a transient
callback. The next callback reports 16 ms since that frame, not zero.
A second paired authored control restarts a previously queued run
before its callback; the canceled run does not tick, and the new run
also first ticks in the next batch. The scheduler snapshots runs before
dispatch and ignores stale run generations. Both lifecycle originals
and the adjacent scheduler phase controls remain green. This does not
claim general ordering of ordinary callbacks relative to tickers.
