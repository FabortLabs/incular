import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Changing controllers keeps the live unsnapped page', (
    WidgetTester tester,
  ) async {
    final first = PageController(viewportFraction: 0.875);
    final second = PageController(initialPage: 4, viewportFraction: 0.75);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    Widget build(PageController controller) => Directionality(
      textDirection: TextDirection.ltr,
      child: PageView(
        controller: controller,
        pageSnapping: false,
        children: List<Widget>.generate(6, (int i) => Text('Page $i')),
      ),
    );

    await tester.pumpWidget(build(first));
    first.jumpTo(1260);
    await tester.pump();
    expect(first.page, closeTo(1.8, 1e-12));

    await tester.pumpWidget(build(second));
    expect(first.hasClients, isFalse);
    expect(second.hasClients, isTrue);
    expect(second.page, closeTo(1.8, 1e-12));
    expect(second.position.pixels, closeTo(1080, 1e-12));
    await tester.pump();
    expect(second.position.pixels, closeTo(1080, 1e-12));
  });
}
