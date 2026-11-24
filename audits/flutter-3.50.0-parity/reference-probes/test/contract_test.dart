import 'dart:ui' as ui;

import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget loose100(Widget child) => Directionality(
  textDirection: TextDirection.ltr,
  child: Center(
    child: SizedBox(
      width: 100,
      height: 30,
      child: Align(alignment: Alignment.topLeft, child: child),
    ),
  ),
);

void main() {
  testWidgets('min Row retains the flexible allocations', (tester) async {
    const row = ValueKey('row'),
        left = ValueKey('left'),
        right = ValueKey('right');
    await tester.pumpWidget(
      loose100(
        const Row(
          key: row,
          mainAxisSize: MainAxisSize.min,
          children: [
            Expanded(child: SizedBox(key: left, width: 10, height: 10)),
            Expanded(child: SizedBox(key: right, width: 10, height: 10)),
          ],
        ),
      ),
    );
    expect(tester.getSize(find.byKey(row)).width, 100);
    expect(
      tester.getTopLeft(find.byKey(right)).dx -
          tester.getTopLeft(find.byKey(left)).dx,
      50,
    );
  });

  testWidgets('default Table allocates flex before measuring children', (
    tester,
  ) async {
    const left = ValueKey('left'), right = ValueKey('right');
    await tester.pumpWidget(
      loose100(
        Table(
          children: const [
            TableRow(
              children: [
                SizedBox(key: left, width: 80, height: 10),
                SizedBox(key: right, width: 80, height: 10),
              ],
            ),
          ],
        ),
      ),
    );
    expect(tester.getSize(find.byKey(left)).width, 50);
    expect(tester.getSize(find.byKey(right)).width, 50);
    expect(
      tester.getTopLeft(find.byKey(right)).dx -
          tester.getTopLeft(find.byKey(left)).dx,
      50,
    );
  });

  testWidgets('MergeSemantics advertises and routes the first tap', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final seen = <String>[];
    try {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: MergeSemantics(
            child: Column(
              children: [
                Semantics(
                  label: 'North',
                  onTap: () => seen.add('North'),
                  child: const SizedBox(width: 60, height: 40),
                ),
                Semantics(
                  label: 'South',
                  onTap: () => seen.add('South'),
                  child: const SizedBox(width: 60, height: 40),
                ),
              ],
            ),
          ),
        ),
      );
      final node = tester.getSemantics(find.byType(MergeSemantics));
      expect(node.getSemanticsData().hasAction(ui.SemanticsAction.tap), isTrue);
      tester.binding.pipelineOwner.semanticsOwner!.performAction(
        node.id,
        ui.SemanticsAction.tap,
      );
      expect(seen, ['North']);
    } finally {
      handle.dispose();
    }
  });

  testWidgets('Ticker started in transient waits until the next batch', (
    tester,
  ) async {
    final ticks = <Duration>[];
    final ticker = Ticker(ticks.add);
    addTearDown(ticker.dispose);
    tester.binding.scheduleFrameCallback((_) {
      ticker.start();
    });
    await tester.pump();
    expect(ticks, isEmpty);
    await tester.pump(const Duration(milliseconds: 16));
    expect(ticks, [const Duration(milliseconds: 16)]);
    ticker.stop();
  });
  testWidgets('Wrap keeps an exact fractional edge on the same run', (
    tester,
  ) async {
    const left = ValueKey('left'), right = ValueKey('right');
    await tester.pumpWidget(
      loose100(
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 20.4),
          child: const Wrap(
            children: [
              SizedBox(key: left, width: 10.1, height: 10),
              SizedBox(key: right, width: 10.3, height: 10),
            ],
          ),
        ),
      ),
    );
    expect(
      tester.getTopLeft(find.byKey(right)).dy -
          tester.getTopLeft(find.byKey(left)).dy,
      0,
    );
  });
  test('ScrollMetrics distinguishes overscroll from the exact edge', () {
    final m = FixedScrollMetrics(
      minScrollExtent: 0,
      maxScrollExtent: 200,
      pixels: -20,
      viewportDimension: 100,
      axisDirection: AxisDirection.down,
      devicePixelRatio: 2,
    );
    expect(m.atEdge, isFalse);
    expect(m.extentInside, 80);
  });
}
