import 'package:flutter/animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('reversing mid-run starts from the unrounded value', (tester) async {
    final controller = AnimationController(
      vsync: tester,
      duration: const Duration(milliseconds: 600),
    );
    addTearDown(controller.dispose);
    controller.forward();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    expect(controller.value, closeTo(1 / 3, 1e-12));

    controller.reverse();
    await tester.pump(); // Anchor the new ticker before advancing its clock.
    expect(controller.value, closeTo(1 / 3, 1e-12));
    await tester.pump(const Duration(milliseconds: 100));
    expect(controller.value, closeTo(1 / 6, 1e-12));
    await tester.pump(const Duration(milliseconds: 100));
    expect(controller.value, 0);
    expect(controller.status, AnimationStatus.reverse);
    await tester.pump(const Duration(milliseconds: 1));
    expect(controller.status, AnimationStatus.dismissed);
  });
}
