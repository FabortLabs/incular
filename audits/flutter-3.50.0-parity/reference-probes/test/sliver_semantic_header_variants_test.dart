import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final bool persistent in <bool>[false, true]) {
    testWidgets(
      'Pinned header exclusion follows its renderer policy (persistent=$persistent)',
      (WidgetTester tester) async {
        const ValueKey<String> headerKey = ValueKey<String>('header');
        const ValueKey<String> bodyKey = ValueKey<String>('body');
        final Widget child = Semantics(
          key: headerKey,
          container: true,
          label: 'header',
          child: const SizedBox(height: 300),
        );
        final ScrollController controller = ScrollController();
        addTearDown(controller.dispose);
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: CustomScrollView(
              controller: controller,
              slivers: <Widget>[
                const SliverToBoxAdapter(child: SizedBox(height: 100)),
                if (persistent)
                  SliverPersistentHeader(pinned: true, delegate: _Header(child))
                else
                  SliverResizingHeader(
                    minExtentPrototype: const SizedBox(height: 300),
                    child: child,
                  ),
                SliverToBoxAdapter(
                  child: Semantics(
                    key: bodyKey,
                    label: 'content',
                    container: true,
                    child: const SizedBox(height: 1600),
                  ),
                ),
              ],
            ),
          ),
        );
        final int initialId = tester.getSemantics(find.byKey(headerKey)).id;
        for (final double offset in <double>[0, 99, 100, 101, 400, 100, 0]) {
          controller.jumpTo(offset);
          await tester.pump();
          final SemanticsNode header = tester.getSemantics(
            find.byKey(headerKey),
          );
          final SemanticsNode pane = tester
              .getSemantics(find.byKey(bodyKey))
              .parent!;
          final SemanticsNode tagged =
              header.isTagged(RenderViewport.useTwoPaneSemantics)
              ? header
              : header.parent!;
          expect(header.id, initialId);
          expect(tagged.isTagged(RenderViewport.useTwoPaneSemantics), isTrue);
          final bool excluded = persistent || offset > 100;
          expect(
            tagged.isTagged(RenderViewport.excludeFromScrolling),
            excluded,
            reason: 'offset $offset',
          );
          expect(tagged.parent?.id, excluded ? pane.parent?.id : pane.id);
          expect(
            tester.getRect(find.byKey(headerKey)),
            Rect.fromLTWH(0, offset < 100 ? 100 - offset : 0, 800, 300),
          );
        }
      },
    );
  }
}

class _Header extends SliverPersistentHeaderDelegate {
  const _Header(this.child);
  final Widget child;

  @override
  double get minExtent => 300;

  @override
  double get maxExtent => 300;

  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) => child;

  @override
  bool shouldRebuild(covariant _Header oldDelegate) =>
      child != oldDelegate.child;
}
