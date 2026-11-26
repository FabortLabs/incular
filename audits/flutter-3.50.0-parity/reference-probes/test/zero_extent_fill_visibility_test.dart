import 'package:flutter/painting.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool bounce in <bool>[true, false]) {
    testWidgets('Zero-extent fill visibility with bounce=$bounce', (
      WidgetTester tester,
    ) async {
      const ValueKey<String> key = ValueKey<String>('zero fill');
      final ScrollController controller = ScrollController();
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            controller: controller,
            physics: bounce
                ? const BouncingScrollPhysics()
                : const ClampingScrollPhysics(),
            slivers: <Widget>[
              SliverFixedExtentList.builder(
                itemExtent: 150,
                itemCount: 5,
                itemBuilder: (BuildContext context, int index) => Semantics(
                  label: '$index',
                  child: const ColoredBox(color: Color(0xFFFF0000)),
                ),
              ),
              const SliverFillRemaining(
                hasScrollBody: false,
                fillOverscroll: true,
                child: ColoredBox(key: key, color: Color(0xFF00FF00)),
              ),
            ],
          ),
        ),
      );

      void check(double pixels, double paintExtent, bool lastRowVisible) {
        final RenderSliverFillRemainingAndOverscroll sliver = tester
            .allRenderObjects
            .whereType<RenderSliverFillRemainingAndOverscroll>()
            .toSet()
            .single;
        expect(controller.offset, pixels);
        expect(sliver.geometry!.paintExtent, paintExtent);
        expect(sliver.geometry!.visible, paintExtent > 0);
        expect(sliver.child!.size, const Size(800, 0));
        expect(
          find.byKey(key),
          paintExtent > 0 ? findsOneWidget : findsNothing,
        );
        expect(find.byKey(key, skipOffstage: false), findsOneWidget);
        expect(
          find.bySemanticsLabel('4'),
          lastRowVisible ? findsOneWidget : findsNothing,
        );
      }

      check(0, 0, false);
      controller.jumpTo(controller.position.maxScrollExtent);
      await tester.pump();
      check(150, 0, true);
      final TestGesture gesture = await tester.startGesture(
        const Offset(400, 300),
      );
      await gesture.moveBy(const Offset(0, -50));
      await tester.pump();
      check(bounce ? 200 : 150, bounce ? 50 : 0, true);
      await gesture.up();
      await tester.pumpAndSettle();
      check(150, 0, true);
    });
  }
}
