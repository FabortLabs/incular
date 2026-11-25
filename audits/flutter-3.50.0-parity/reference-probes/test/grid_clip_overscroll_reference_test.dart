import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('fitting grid reports overflow while overscrolled only', (
    WidgetTester tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: GridView.count(
          crossAxisCount: 3,
          controller: controller,
          physics: const BouncingScrollPhysics(
            parent: AlwaysScrollableScrollPhysics(),
          ),
          children: const [SizedBox(width: 50, height: 50)],
        ),
      ),
    );
    final RenderViewport viewport = tester.allRenderObjects
        .whereType<RenderViewport>()
        .first;
    expect(viewport.clipBehavior, Clip.hardEdge);
    expect(viewport.hasVisualOverflow, isFalse);
    controller.jumpTo(-12);
    await tester.pump();
    expect(viewport.hasVisualOverflow, isTrue);
    controller.jumpTo(0);
    await tester.pump();
    expect(viewport.hasVisualOverflow, isFalse);
  });
}
