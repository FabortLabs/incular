import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const alignments = [
    Alignment.topLeft,
    Alignment.topRight,
    Alignment.center,
    Alignment.bottomLeft,
    Alignment.bottomRight,
  ];
  testWidgets('all 25 applied anchor pairs locate the opaque tap target', (
    WidgetTester tester,
  ) async {
    final link = LayerLink();
    final child = GlobalKey();
    var taps = 0;
    for (final target in alignments) {
      for (final follower in alignments) {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Stack(
              children: [
                Positioned(
                  left: 123,
                  top: 456,
                  child: CompositedTransformTarget(
                    link: link,
                    child: const SizedBox.square(dimension: 10),
                  ),
                ),
                CompositedTransformFollower(
                  link: link,
                  targetAnchor: target,
                  followerAnchor: follower,
                  child: GestureDetector(
                    behavior: HitTestBehavior.opaque,
                    onTap: () => taps++,
                    child: SizedBox.square(key: child, dimension: 2),
                  ),
                ),
              ],
            ),
          ),
        );
        final anchor = const Offset(123, 456) +
            target.alongSize(const Size(10, 10)) -
            follower.alongSize(const Size(2, 2));
        final box = child.currentContext!.findRenderObject()! as RenderBox;
        expect(box.size, const Size(2, 2));
        expect(box.localToGlobal(Offset.zero), anchor);
        final before = taps;
        await tester.tapAt(anchor + const Offset(1, 1));
        expect(taps, before + 1);
      }
    }
  });
}
