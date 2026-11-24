import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('horizontal reverse starts from ambient direction and retains pixels', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    const viewport = ValueKey('viewport');
    const content = ValueKey('content');
    Widget scene(TextDirection direction, bool reverse) => Directionality(
      textDirection: direction,
      child: Align(alignment: Alignment.topLeft, child: SizedBox(
        key: viewport,
        width: 100,
        height: 60,
        child: SingleChildScrollView(
          controller: controller,
          scrollDirection: Axis.horizontal,
          reverse: reverse,
          child: const SizedBox(key: content, width: 200, height: 60),
        ),
      )),
    );
    double x() => tester.getTopLeft(find.byKey(content)).dx -
        tester.getTopLeft(find.byKey(viewport)).dx;
    for (final (direction, reverse, expected) in [
      (TextDirection.ltr, false, 0.0),
      (TextDirection.ltr, true, -100.0),
      (TextDirection.rtl, false, -100.0),
      (TextDirection.rtl, true, 0.0),
    ]) {
      await tester.pumpWidget(scene(direction, reverse));
      expect(controller.offset, 0.0);
      expect(x(), expected);
    }
    await tester.pumpWidget(scene(TextDirection.ltr, false));
    controller.jumpTo(25);
    await tester.pump();
    expect(x(), -25.0);
    for (final (direction, expected) in [
      (TextDirection.rtl, -75.0), (TextDirection.ltr, -25.0),
    ]) {
      await tester.pumpWidget(scene(direction, false));
      expect(controller.offset, 25.0);
      expect(x(), expected);
    }
  });
}
