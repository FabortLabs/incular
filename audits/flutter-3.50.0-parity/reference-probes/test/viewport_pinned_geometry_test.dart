import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class HeaderDelegate extends SliverPersistentHeaderDelegate {
  @override
  double get minExtent => 100;
  @override
  double get maxExtent => 300;
  @override
  bool shouldRebuild(covariant HeaderDelegate oldDelegate) => false;
  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) => const SizedBox.expand(key: ValueKey('pinned'));
}

void main() {
  testWidgets(
    'Pinned header and the box after it reveal at exact coordinates',
    (WidgetTester tester) async {
      final ScrollController controller = ScrollController(
        initialScrollOffset: 300,
      );
      addTearDown(controller.dispose);
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            controller: controller,
            slivers: List<Widget>.generate(20, (int index) {
              return index == 10
                  ? SliverPersistentHeader(
                      pinned: true,
                      delegate: HeaderDelegate(),
                    )
                  : SliverToBoxAdapter(
                      child: SizedBox(key: ValueKey(index), height: 300),
                    );
            }),
          ),
        ),
      );
      controller.jumpTo(4500);
      await tester.pumpAndSettle();
      final Finder header = find.byKey(const ValueKey('pinned'));
      expect(tester.getRect(header), const Rect.fromLTWH(0, 0, 800, 100));
      tester.renderObject(header).showOnScreen();
      await tester.pumpAndSettle();
      expect(controller.offset, 4500);

      for (final (double pixels, double top) in <(double, double)>[
        (3600, -300),
        (3400, -100),
      ]) {
        controller.jumpTo(pixels);
        await tester.pumpAndSettle();
        final Finder next = find.byKey(const ValueKey(11), skipOffstage: false);
        expect(tester.getTopLeft(next).dy, top);
        tester.renderObject(next).showOnScreen();
        await tester.pumpAndSettle();
        expect(controller.offset, 3200);
        expect(tester.getTopLeft(next).dy, 100);
      }
    },
  );
}
