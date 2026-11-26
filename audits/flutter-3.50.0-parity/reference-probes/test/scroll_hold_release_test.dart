import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final cancelled in [false, true]) {
    testWidgets(
      'ending an overscroll hold settles without another drag (cancelled: $cancelled)',
      (tester) async {
        final controller = ScrollController();
        addTearDown(controller.dispose);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: ListView.builder(
              controller: controller,
              physics: const BouncingScrollPhysics(),
              itemExtent: 200,
              itemCount: 250,
              itemBuilder: (context, index) => Text('row $index'),
            ),
          ),
        );
        await tester.flingFrom(
          const Offset(400, 50),
          const Offset(0, 500),
          1000,
        );
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 100));
        final previous = controller.offset;
        expect(previous, lessThan(0.0));
        await tester.pump(const Duration(milliseconds: 100));
        final held = controller.offset;
        expect(held, greaterThan(previous));
        expect(held, lessThan(0.0));

        final contact = await tester.startGesture(const Offset(400, 50));
        await tester.pump(const Duration(milliseconds: 400));
        expect(controller.offset, held);
        if (cancelled) {
          await contact.cancel();
        } else {
          await contact.up();
        }
        await tester.pumpAndSettle();
        expect(controller.offset, 0.0);
      },
    );
  }
}
