import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A scrollbar remains interactive over a negative-only range', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    const center = ValueKey<int>(1);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: MediaQuery(
          data: const MediaQueryData(),
          child: RawScrollbar(
            controller: controller,
            thumbVisibility: true,
            child: CustomScrollView(
              controller: controller,
              center: center,
              slivers: const [
                SliverToBoxAdapter(child: SizedBox(height: 600)),
                SliverToBoxAdapter(key: center, child: SizedBox(height: 600)),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(controller.position.minScrollExtent, -600);
    expect(controller.position.maxScrollExtent, 0);

    void expectThumb(double top) {
      expect(
        find.byType(RawScrollbar),
        paints
          ..rect(rect: const Rect.fromLTRB(794, 0, 800, 600))
          ..rect(
            rect: Rect.fromLTWH(794, top, 6, 300),
            color: const Color(0x66BCBCBC),
          ),
      );
    }

    expectThumb(300);
    final drag = await tester.startGesture(const Offset(797, 450));
    await tester.pumpAndSettle();
    await drag.moveBy(const Offset(0, -10));
    await tester.pumpAndSettle();
    await drag.up();
    await tester.pumpAndSettle();
    expect(controller.offset, closeTo(-20, 0.001));
    expectThumb(290);
  });
}
