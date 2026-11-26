import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _NarrowClip extends CustomClipper<Rect> {
  const _NarrowClip();

  @override
  Rect getClip(Size size) => Rect.fromLTRB(0, 10, size.width / 2, size.height);

  @override
  bool shouldReclip(_NarrowClip oldClipper) => false;
}

void main() {
  testWidgets('nested sliver clipping intersects the two hit boundaries', (
    tester,
  ) async {
    const outerKey = ValueKey('outer');
    const rowKey = ValueKey('row');
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          slivers: [
            SliverClipRect(
              key: outerKey,
              clipper: const _NarrowClip(),
              sliver: SliverClipRRect(
                borderRadius: BorderRadius.circular(20),
                clipBehavior: Clip.antiAliasWithSaveLayer,
                sliver: SliverFixedExtentList.builder(
                  itemExtent: 40,
                  itemCount: 5,
                  itemBuilder: (context, index) => ColoredBox(
                    key: index == 0 ? rowKey : null,
                    color: const Color(0xFF224488),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    final outer = tester.renderObject<RenderSliverClipRect>(
      find.byKey(outerKey),
    );
    final row = tester.renderObject<RenderBox>(find.byKey(rowKey));
    bool hit(double x, double y) => outer.hitTest(
      SliverHitTestResult(),
      mainAxisPosition: y,
      crossAxisPosition: x,
    );
    expect(hit(50, 20), isTrue);
    expect(hit(450, 20), isFalse);
    expect(hit(0, 10), isFalse);

    outer.clipBehavior = Clip.none;
    await tester.pump();
    expect(tester.renderObject<RenderBox>(find.byKey(rowKey)), same(row));
    expect(hit(450, 20), isTrue);
    expect(hit(0, 10), isFalse);
  });
}
