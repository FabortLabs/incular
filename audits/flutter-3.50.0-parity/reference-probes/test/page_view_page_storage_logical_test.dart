import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('PageStorage restores the logical page, not old pixels', (
    WidgetTester tester,
  ) async {
    final bucket = PageStorageBucket();
    final first = PageController(viewportFraction: 0.75);
    final restored = PageController(initialPage: 1, viewportFraction: 0.5);
    final disabled = PageController(
      initialPage: 1,
      viewportFraction: 0.5,
      keepPage: false,
    );
    final finalRestore = PageController(viewportFraction: 0.5);
    addTearDown(first.dispose);
    addTearDown(restored.dispose);
    addTearDown(disabled.dispose);
    addTearDown(finalRestore.dispose);

    Widget build(PageController? controller, double width) => Directionality(
      textDirection: TextDirection.ltr,
      child: PageStorage(
        bucket: bucket,
        child: Center(
          child: SizedBox(
            width: width,
            height: 300,
            child: controller == null
                ? const SizedBox()
                : PageView(
                    key: const PageStorageKey<String>('logical page'),
                    controller: controller,
                    children: List<Widget>.generate(5, (i) => Text('Page $i')),
                  ),
          ),
        ),
      ),
    );

    await tester.pumpWidget(build(first, 400));
    first.jumpToPage(2);
    await tester.pumpAndSettle();
    expect(first.position.pixels, 600);
    await tester.pumpWidget(build(null, 400));
    expect(first.hasClients, isFalse);
    expect(() => first.page, throwsAssertionError);

    await tester.pumpWidget(build(restored, 500));
    expect(restored.page, 2);
    expect(restored.position.pixels, 500);
    await tester.pumpWidget(build(null, 500));

    await tester.pumpWidget(build(disabled, 500));
    expect(disabled.page, 1);
    expect(disabled.position.pixels, 250);
    disabled.jumpToPage(3);
    await tester.pumpAndSettle();
    await tester.pumpWidget(build(null, 500));

    await tester.pumpWidget(build(finalRestore, 500));
    expect(finalRestore.page, 2);
    expect(finalRestore.position.pixels, 500);
  });
}
