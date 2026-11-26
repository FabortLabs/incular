import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _WatchedRectangle extends CustomClipper<Rect> {
  _WatchedRectangle(this.top) : super(reclip: top);

  final ValueNotifier<double> top;
  int calls = 0;

  @override
  Rect getClip(Size size) {
    calls++;
    return Rect.fromLTRB(0, top.value, size.width, size.height);
  }

  @override
  bool shouldReclip(_WatchedRectangle oldClipper) => false;
}

void main() {
  testWidgets('sliver live clip updates paint and hit in place', (
    tester,
  ) async {
    final top = ValueNotifier<double>(50);
    addTearDown(top.dispose);
    final clipper = _WatchedRectangle(top);
    const key = ValueKey('content');
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          slivers: [
            SliverClipRect(
              clipper: clipper,
              sliver: const SliverToBoxAdapter(
                child: SizedBox(
                  key: key,
                  height: 100,
                  child: ColoredBox(color: Color(0xFF224488)),
                ),
              ),
            ),
          ],
        ),
      ),
    );
    final render = tester.renderObject<RenderSliverClipRect>(
      find.byType(SliverClipRect),
    );
    final box = tester.renderObject<RenderBox>(find.byKey(key));
    bool hits() => render.hitTest(
      SliverHitTestResult(),
      mainAxisPosition: 40,
      crossAxisPosition: 400,
    );
    expect(hits(), isFalse);
    expect(
      render.paint,
      paints..clipRect(rect: const Rect.fromLTRB(0, 50, 800, 100)),
    );

    final calls = clipper.calls;
    top.value = 30;
    expect(hits(), isTrue);
    await tester.pump();
    expect(tester.renderObject<RenderBox>(find.byKey(key)), same(box));
    expect(
      render.paint,
      paints..clipRect(rect: const Rect.fromLTRB(0, 30, 800, 100)),
    );
    expect(clipper.calls, calls + 1);
  });
}
