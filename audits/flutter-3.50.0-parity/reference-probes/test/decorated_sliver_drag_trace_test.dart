import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class StretchingHeader extends SliverPersistentHeaderDelegate {
  @override
  double get minExtent => 0;

  @override
  double get maxExtent => 200;

  @override
  OverScrollHeaderStretchConfiguration get stretchConfiguration =>
      OverScrollHeaderStretchConfiguration();

  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) {
    return const SizedBox(width: 300, height: 200);
  }

  @override
  bool shouldRebuild(covariant SliverPersistentHeaderDelegate oldDelegate) =>
      false;
}

void main() {
  testWidgets(
    'the two touch samples leading to the stretched decorated header',
    (tester) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      const key = ValueKey('header-border');
      await tester.pumpWidget(
        TestWidgetsApp(
          home: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 300,
              height: 500,
              child: CustomScrollView(
                controller: controller,
                physics: const BouncingScrollPhysics(),
                slivers: [
                  DecoratedSliver(
                    key: key,
                    decoration: BoxDecoration(
                      border: Border.all(
                        strokeAlign: BorderSide.strokeAlignCenter,
                      ),
                    ),
                    sliver: SliverPersistentHeader(
                      pinned: true,
                      delegate: StretchingHeader(),
                    ),
                  ),
                  const SliverToBoxAdapter(child: SizedBox(height: 1000)),
                ],
              ),
            ),
          ),
        ),
      );
      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(CustomScrollView)),
      );
      await gesture.moveBy(const Offset(0, 20));
      final first = controller.offset;
      await gesture.moveBy(const Offset(0, 25));
      final second = controller.offset;
      await gesture.up();
      final released = controller.offset;
      await tester.pump();
      expect(first, closeTo(-3.5 / 3, 0.0001));
      expect(second, closeTo(-25.07923674463331, 0.0001));
      expect(released, second);
      expect(controller.offset, second);
      expect(
        find.byKey(key),
        paints..something((method, arguments) {
          if (method != #drawRect) return false;
          final rect = arguments[0] as Rect;
          final paint = arguments[1] as Paint;
          expect(rect.left, 0);
          expect(rect.top, 0);
          expect(rect.width, 300);
          expect(rect.height, closeTo(200 - second, 0.03));
          expect(paint.style, PaintingStyle.stroke);
          expect(paint.strokeWidth, 1);
          return true;
        }),
      );
    },
  );
}
