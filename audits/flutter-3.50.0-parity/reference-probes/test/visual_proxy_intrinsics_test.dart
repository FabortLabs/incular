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
  const target = ValueKey<String>('proxy');
  const child = _PreferredExtent(child: SizedBox(width: 55, height: 110));
  final wrappers = <(String, Widget Function(Widget))>[
    (
      'color',
      (child) =>
          ColoredBox(key: target, color: const Color(0xffffffff), child: child),
    ),
    (
      'transparent opacity',
      (child) => Opacity(key: target, opacity: 0, child: child),
    ),
    ('clip', (child) => ClipRect(key: target, child: child)),
    ('ignore pointer', (child) => IgnorePointer(key: target, child: child)),
    ('repaint boundary', (child) => RepaintBoundary(key: target, child: child)),
  ];
  for (final (name, wrap) in wrappers) {
    testWidgets(
      '$name forwards intrinsic sizes without using ordinary child layout',
      (tester) async {
        Future<void> pump(Widget child) => tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Center(child: child),
          ),
        );

        await pump(wrap(child));
        final finder = find.byKey(target);
        final render = tester.renderObject<RenderBox>(finder);
        expect(render.getMinIntrinsicWidth(50), 55);
        expect(render.getMaxIntrinsicWidth(50), 100);
        expect(render.getMinIntrinsicHeight(50), 110);
        expect(render.getMaxIntrinsicHeight(50), 200);
        expect(tester.getSize(finder), const Size(55, 110));

        await pump(IntrinsicWidth(child: wrap(child)));
        expect(tester.getSize(finder), const Size(100, 110));
        await pump(IntrinsicHeight(child: wrap(child)));
        expect(tester.getSize(finder), const Size(55, 200));
      },
    );
  }
}
