import 'package:flutter/animation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget _pages(PageController controller, {double? width}) => Directionality(
  textDirection: TextDirection.ltr,
  child: Center(
    child: SizedBox(
      width: width,
      child: PageView(
        controller: controller,
        children: const [Text('one'), Text('two'), Text('three'), Text('four')],
      ),
    ),
  ),
);

Future<void> _animate(PageController controller, int target) =>
    controller.animateToPage(
      target,
      duration: const Duration(milliseconds: 150),
      curve: Curves.ease,
    );

void main() {
  testWidgets('Interrupting a driven page animation resolves its own future', (
    tester,
  ) async {
    final controller = PageController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(_pages(controller));
    var resolved = false;
    final interrupted = _animate(controller, 3).then((_) => resolved = true);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(resolved, isFalse);
    controller.jumpToPage(2);
    await tester.pump();
    expect(resolved, isTrue);
    await interrupted;
    expect(controller.page, 2);
  });

  testWidgets(
    'A page deferred in a zero-width viewport completes immediately',
    (tester) async {
      final controller = PageController();
      addTearDown(controller.dispose);
      await tester.pumpWidget(_pages(controller, width: 0));
      var resolved = false;
      final motion = _animate(controller, 2).then((_) => resolved = true);
      await tester.pump();
      expect(resolved, isTrue);
      await motion;
      await tester.pumpWidget(_pages(controller, width: 800));
      expect(controller.page, 2);
    },
  );
}
