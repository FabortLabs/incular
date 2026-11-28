import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'An omitted overscroll floor inherits the configured thumb minimum',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: MediaQuery(
            data: const MediaQueryData(),
            child: RawScrollbar(
              controller: controller,
              thumbVisibility: true,
              minThumbLength: 36,
              child: SingleChildScrollView(
                physics: const BouncingScrollPhysics(),
                controller: controller,
                child: const SizedBox(height: 50000),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      const ink = Color(0x66BCBCBC);
      final scrollbar = find.byType(RawScrollbar);
      expect(
        scrollbar,
        paints
          ..rect(
            rect: const Rect.fromLTRB(794, 0, 800, 600),
            color: const Color(0x00000000),
          )
          ..rect(rect: const Rect.fromLTRB(794, 0, 800, 36), color: ink),
      );

      final gesture = await tester.startGesture(tester.getCenter(scrollbar));
      await gesture.moveBy(const Offset(0, 1000));
      await tester.pump();
      expect(controller.offset, lessThan(0));
      expect(
        scrollbar,
        paints
          ..rect(
            rect: const Rect.fromLTRB(794, 0, 800, 600),
            color: const Color(0x00000000),
          )
          ..rect(rect: const Rect.fromLTRB(794, 0, 800, 36), color: ink),
      );
      await gesture.up();
      await tester.pumpAndSettle();
    },
  );
}
