import 'package:flutter/painting.dart';
import 'package:flutter/rendering.dart' show RenderProxyBox;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _RecordingStadium extends StadiumBorder {
  _RecordingStadium(this.bounds);

  final List<Rect> bounds;

  @override
  Path getOuterPath(Rect rect, {TextDirection? textDirection}) {
    bounds.add(rect);
    return super.getOuterPath(rect, textDirection: textDirection);
  }
}

void main() {
  testWidgets(
    'Clip widgets retain their render object as a child comes and goes',
    (WidgetTester tester) async {
      for (final kind in [ClipRect, ClipRRect, ClipPath]) {
        RenderProxyBox? retained;
        for (final hasChild in [false, true, false, true]) {
          final child = hasChild
              ? const ColoredBox(color: Color(0xffffffff))
              : null;
          final Widget clip = kind == ClipRect
              ? ClipRect(child: child)
              : kind == ClipRRect
              ? ClipRRect(child: child)
              : ClipPath(child: child);
          await tester.pumpWidget(
            Center(child: SizedBox.square(dimension: 40, child: clip)),
          );
          final render = tester.renderObject<RenderProxyBox>(find.byType(kind));
          retained ??= render;
          expect(render, same(retained));
          expect(render.child != null, hasChild);
        }
      }
    },
  );

  testWidgets('ClipPath.shape uses its child size under loose constraints', (
    WidgetTester tester,
  ) async {
    final bounds = <Rect>[];
    await tester.pumpWidget(
      Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 300, maxHeight: 300),
          child: ClipPath.shape(
            shape: _RecordingStadium(bounds),
            child: const SizedBox(width: 100, height: 40, child: Placeholder()),
          ),
        ),
      ),
    );
    expect(tester.getSize(find.byType(ClipPath)), const Size(100, 40));
    expect(bounds, <Rect>[const Rect.fromLTWH(0, 0, 100, 40)]);
  });
}
