import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget fixture(Widget child) => Directionality(
  textDirection: TextDirection.ltr,
  child: Center(child: SizedBox(width: 100, height: 100, child: child)),
);

void main() {
  testWidgets(
    'pinned overlap moves the physical leading edge of the child clip',
    (tester) async {
      const full = Rect.fromLTWH(0, 0, 100, 100);
      for (final axis in Axis.values) {
        for (final reverse in [false, true]) {
          for (final clip in [Clip.hardEdge, Clip.none]) {
            final controller = ScrollController();
            addTearDown(controller.dispose);
            await tester.pumpWidget(
              fixture(
                CustomScrollView(
                  key: UniqueKey(),
                  controller: controller,
                  scrollDirection: axis,
                  reverse: reverse,
                  clipBehavior: clip,
                  slivers: [
                    const PinnedHeaderSliver(
                      child: SizedBox(width: 20, height: 20),
                    ),
                    SliverToBoxAdapter(
                      child: SizedBox(
                        width: axis == Axis.horizontal ? 220 : 20,
                        height: axis == Axis.vertical ? 220 : 20,
                      ),
                    ),
                  ],
                ),
              ),
            );
            controller.jumpTo(60);
            await tester.pump();
            final viewport = tester.allRenderObjects
                .whereType<RenderViewport>()
                .single;
            final header = viewport.firstChild!;
            final content = viewport.childAfter(header)!;
            final edge = reverse ? 0.0 : 20.0;
            final adjusted = axis == Axis.vertical
                ? Rect.fromLTWH(0, edge, 100, 80)
                : Rect.fromLTWH(edge, 0, 80, 100);
            expect(
              viewport.describeApproximatePaintClip(header),
              clip == Clip.none ? null : full,
            );
            expect(
              viewport.describeApproximatePaintClip(content),
              clip == Clip.none ? null : adjusted,
              reason: '$axis, reverse=$reverse, clip=$clip',
            );
          }
        }
      }
    },
  );

  testWidgets(
    'an explicitly ensured sliver is unclipped only outside paint and cache',
    (tester) async {
      for (final cache in [0.0, 150.0]) {
        await tester.pumpWidget(
          fixture(
            CustomScrollView(
              key: UniqueKey(),
              scrollCacheExtent: ScrollCacheExtent.pixels(cache),
              slivers: const [
                SliverToBoxAdapter(child: SizedBox(height: 220)),
                SliverEnsureSemantics(
                  sliver: SliverToBoxAdapter(child: SizedBox(height: 20)),
                ),
              ],
            ),
          ),
        );
        final viewport = tester.allRenderObjects
            .whereType<RenderViewport>()
            .single;
        final ensured = viewport.childAfter(viewport.firstChild!)!;
        expect(
          viewport.describeApproximatePaintClip(ensured),
          cache == 0 ? null : const Rect.fromLTWH(0, 0, 100, 100),
        );
      }
    },
  );
}
