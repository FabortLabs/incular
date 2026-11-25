import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class Header extends SliverPersistentHeaderDelegate {
  @override
  double get minExtent => 100;
  @override
  double get maxExtent => 200;
  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) => const SizedBox.expand();
  @override
  bool shouldRebuild(Header oldDelegate) => false;
}

void main() {
  for (final axis in Axis.values) {
    for (final reverse in [false, true]) {
      for (final pinned in [false, true]) {
        testWidgets(
          'ensureVisible with pinned=$pinned, $axis, reverse=$reverse',
          (tester) async {
            await tester.binding.setSurfaceSize(const Size(600, 600));
            final controller = ScrollController();
            final key = GlobalKey();
            Widget mainBox(double extent, {Key? key}) => SizedBox(
              key: key,
              height: axis == Axis.vertical ? extent : null,
              width: axis == Axis.horizontal ? extent : null,
            );
            await tester.pumpWidget(
              Directionality(
                textDirection: TextDirection.ltr,
                child: CustomScrollView(
                  scrollDirection: axis,
                  reverse: reverse,
                  controller: controller,
                  slivers: [
                    SliverPersistentHeader(
                      delegate: Header(),
                      pinned: pinned,
                      floating: true,
                    ),
                    SliverToBoxAdapter(child: mainBox(300)),
                    SliverToBoxAdapter(child: mainBox(100, key: key)),
                    SliverToBoxAdapter(child: mainBox(1200)),
                  ],
                ),
              ),
            );
            final expected = pinned
                ? <double>[400, 200, 0]
                : <double>[500, 250, 0];
            for (int i = 0; i < 3; i++) {
              await Scrollable.ensureVisible(
                key.currentContext!,
                alignment: i / 2,
              );
              await tester.pump();
              expect(controller.offset, expected[i]);
            }
            await tester.pumpWidget(const SizedBox());
            controller.dispose();
            await tester.binding.setSurfaceSize(null);
          },
        );
      }
    }
  }
}
