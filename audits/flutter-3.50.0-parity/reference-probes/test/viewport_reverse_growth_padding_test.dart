import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('padding follows growth on either side of a forward viewport', (
    WidgetTester tester,
  ) async {
    const center = ValueKey('center sliver');
    const target = ValueKey('leading box');
    for (final axis in Axis.values) {
      final vertical = axis == Axis.vertical;
      final padding = vertical
          ? const EdgeInsets.only(top: 22, bottom: 23)
          : const EdgeInsets.only(left: 22, right: 23);
      Widget sliver({Key? key, Key? boxKey}) => SliverPadding(
        key: key,
        padding: padding,
        sliver: SliverToBoxAdapter(
          child: SizedBox(
            key: boxKey,
            width: vertical ? 300 : 100,
            height: vertical ? 100 : 200,
          ),
        ),
      );
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: SizedBox(
              width: 300,
              height: 200,
              child: CustomScrollView(
                scrollDirection: axis,
                center: center,
                slivers: [sliver(boxKey: target), sliver(key: center)],
              ),
            ),
          ),
        ),
      );
      final RenderBox box = tester.renderObject(
        find.byKey(target, skipOffstage: false),
      );
      final viewport = RenderAbstractViewport.of(box);
      const subrect = Rect.fromLTRB(1, 2, 3, 4);
      expect(viewport.getOffsetToReveal(box, 0).offset, -123);
      expect(viewport.getOffsetToReveal(box, 1).offset, vertical ? -223 : -323);
      expect(
        viewport.getOffsetToReveal(box, 0, rect: subrect).offset,
        vertical ? -121 : -122,
      );
      expect(
        viewport.getOffsetToReveal(box, 1, rect: subrect).offset,
        vertical ? -319 : -420,
      );
    }
  });
}
