import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Inset extends CustomClipper<Rect> {
  const _Inset(this.top, {this.invalidate = false});

  final double top;
  final bool invalidate;

  @override
  Rect getClip(Size size) => Rect.fromLTRB(0, top, size.width, size.height);

  @override
  bool shouldReclip(_Inset oldClipper) => invalidate;
}

void main() {
  testWidgets('a replacement clipper can keep cached geometry until layout', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          controller: controller,
          slivers: const [
            SliverClipRect(
              clipper: _Inset(50),
              sliver: SliverToBoxAdapter(
                child: SizedBox(
                  height: 100,
                  child: ColoredBox(color: Color(0xFF224488)),
                ),
              ),
            ),
            SliverToBoxAdapter(child: SizedBox(height: 1000)),
          ],
        ),
      ),
    );
    final render = tester.renderObject<RenderSliverClipRect>(
      find.byType(SliverClipRect),
    );
    expect(
      render.paint,
      paints..clipRect(rect: const Rect.fromLTRB(0, 50, 800, 100)),
    );

    render.clipper = const _Inset(30);
    await tester.pump();
    expect(
      render.paint,
      paints..clipRect(rect: const Rect.fromLTRB(0, 50, 800, 100)),
    );

    controller.jumpTo(10);
    await tester.pump();
    expect(
      render.paint,
      paints..clipRect(rect: const Rect.fromLTRB(0, 20, 800, 90)),
    );

    render.clipper = const _Inset(55, invalidate: true);
    await tester.pump();
    expect(
      render.paint,
      paints..clipRect(rect: const Rect.fromLTRB(0, 45, 800, 90)),
    );
  });
}
