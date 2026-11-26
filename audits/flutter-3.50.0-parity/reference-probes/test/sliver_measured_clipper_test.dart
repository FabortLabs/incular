import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _LoggedClip extends CustomClipper<Rect> {
  final sizes = <Size>[];

  @override
  Rect getClip(Size size) {
    sizes.add(size);
    return Offset.zero & size;
  }

  @override
  bool shouldReclip(_LoggedClip oldClipper) => false;
}

void main() {
  testWidgets('sliver clipper sees the committed paint size once', (
    tester,
  ) async {
    final clip = _LoggedClip();
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          slivers: [
            SliverClipRect(
              clipper: clip,
              sliver: const SliverToBoxAdapter(
                child: SizedBox(
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
    expect(
      render.paint,
      paints..clipRect(rect: const Rect.fromLTRB(0, 0, 800, 100)),
    );
    expect(clip.sizes, [const Size(800, 100)]);
  });
}
