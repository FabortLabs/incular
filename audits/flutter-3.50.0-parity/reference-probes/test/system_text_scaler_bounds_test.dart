import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('System bounds remain distinct at the same effective scale', (tester) async {
    final TextScaler system = MediaQueryData.fromView(tester.view).textScaler;
    final TextScaler overlap = system
        .clamp(minScaleFactor: 1, maxScaleFactor: 3)
        .clamp(minScaleFactor: 2, maxScaleFactor: 4);
    expect(overlap.scale(17), 34);
    expect(overlap, system.clamp(minScaleFactor: 2, maxScaleFactor: 3));
    expect(overlap, isNot(system.clamp(minScaleFactor: 2, maxScaleFactor: 4)));
    expect(overlap, isNot(TextScaler.linear(2)));
  });
}
