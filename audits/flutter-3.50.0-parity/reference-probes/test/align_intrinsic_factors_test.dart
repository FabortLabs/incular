import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _PreferredExtent extends SingleChildRenderObjectWidget {
  const _PreferredExtent({required super.child});

  @override
  RenderProxyBox createRenderObject(BuildContext context) =>
      _RenderPreferredExtent();
}

class _RenderPreferredExtent extends RenderProxyBox {
  @override
  double computeMaxIntrinsicWidth(double height) => 100;

  @override
  double computeMaxIntrinsicHeight(double width) => 200;
}

void main() {
  testWidgets(
    'Align forwards the cross extent to AspectRatio before applying its own factor',
    (tester) async {
      const target = ValueKey<String>('factored align');
      await tester.pumpWidget(
        const Directionality(
          textDirection: TextDirection.ltr,
          child: Center(
            child: Align(
              key: target,
              widthFactor: 0.5,
              heightFactor: 3,
              child: AspectRatio(aspectRatio: 2, child: SizedBox.shrink()),
            ),
          ),
        ),
      );
      final box = tester.renderObject<RenderBox>(find.byKey(target));
      final before = box.size;
      for (final cross in [10.0, 20.0]) {
        expect(box.getMinIntrinsicWidth(cross), cross);
        expect(box.getMaxIntrinsicWidth(cross), cross);
        expect(box.getMinIntrinsicHeight(cross), cross * 1.5);
        expect(box.getMaxIntrinsicHeight(cross), cross * 1.5);
      }
      expect(box.size, before);
    },
  );

  testWidgets(
    'intrinsic parents see the Align child reported extent, not its layout size',
    (tester) async {
      const target = ValueKey<String>('factored align');
      const align = Align(
        key: target,
        widthFactor: 0.5,
        heightFactor: 0.5,
        child: _PreferredExtent(child: SizedBox(width: 55, height: 110)),
      );
      Future<void> check(Widget child, Size expected) async {
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Center(child: child),
          ),
        );
        expect(tester.getSize(find.byKey(target)), expected);
      }

      await check(const IntrinsicWidth(child: align), const Size(50, 55));
      await check(const IntrinsicHeight(child: align), const Size(27.5, 100));
    },
  );
}
