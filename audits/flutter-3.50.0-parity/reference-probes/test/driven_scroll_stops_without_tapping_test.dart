import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a stop tap never delivers a press to a moving row', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    var presses = 0;
    var taps = 0;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: ListView.builder(
          controller: controller,
          itemExtent: 40,
          itemCount: 100,
          itemBuilder: (context, index) => Listener(
            onPointerDown: (_) => presses += 1,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => taps += 1,
              child: const SizedBox.expand(),
            ),
          ),
        ),
      ),
    );
    const point = Offset(400, 300);
    await tester.tapAt(point);
    await tester.pump(const Duration(milliseconds: 50));
    expect((presses, taps), (1, 1));

    final scroll = controller.animateTo(
      400,
      duration: const Duration(seconds: 1),
      curve: Curves.linear,
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    final stopped = controller.offset;
    expect(stopped, greaterThan(0.0));
    expect(stopped, lessThan(400.0));

    await tester.tapAt(point);
    await tester.pump(const Duration(milliseconds: 50));
    await scroll;
    expect((presses, taps), (1, 1));
    expect(controller.offset, stopped);
    expect(controller.position.isScrollingNotifier.value, isFalse);

    await tester.tapAt(point);
    await tester.pump(const Duration(milliseconds: 50));
    expect((presses, taps), (2, 2));
  });
}
