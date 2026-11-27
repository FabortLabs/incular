import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:widget_preview_scaffold/src/widget_preview_rendering.dart';

void main() {
  testWidgets(
    'preview zoom enlarges both extents without reflowing the child',
    (tester) async {
      final controller = TransformationController();
      addTearDown(controller.dispose);
      final hits = <Offset>[];
      const child = ValueKey('child');
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 120,
              height: 100,
              child: ZoomablePreviewArea(
                transformationController: controller,
                errorThrownDuringTreeConstruction: false,
                child: Listener(
                  behavior: HitTestBehavior.opaque,
                  onPointerDown: (event) => hits.add(event.localPosition),
                  child: const SizedBox(key: child, width: 100, height: 90),
                ),
              ),
            ),
          ),
        ),
      );
      final states = tester
          .stateList<ScrollableState>(find.byType(Scrollable))
          .toList();
      final outer = states
          .singleWhere((s) => s.axisDirection == AxisDirection.down)
          .position;
      final inner = states
          .singleWhere((s) => s.axisDirection == AxisDirection.right)
          .position;
      expect(inner.maxScrollExtent, 0);
      expect(outer.maxScrollExtent, 0);
      controller.value = Matrix4.diagonal3Values(2, 2, 1);
      await tester.pumpAndSettle();
      expect(tester.getSize(find.byKey(child)), const Size(100, 90));
      expect(inner.maxScrollExtent, 80);
      expect(outer.maxScrollExtent, 80);
      await tester.tapAt(const Offset(110, 60));
      expect(hits, [const Offset(55, 30)]);
      inner.jumpTo(50);
      outer.jumpTo(30);
      await tester.pumpAndSettle();
      await tester.tapAt(const Offset(110, 60));
      expect(hits, [const Offset(55, 30), const Offset(80, 45)]);
      controller.value = Matrix4.identity();
      await tester.pumpAndSettle();
      expect(inner.maxScrollExtent, 0);
      expect(outer.maxScrollExtent, 0);
      expect(inner.pixels, 0);
      expect(outer.pixels, 0);
    },
  );
}
