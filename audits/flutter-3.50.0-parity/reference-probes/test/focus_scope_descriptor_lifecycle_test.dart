import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('building a disabled scope does not disable the live scope', (
    tester,
  ) async {
    final scope = FocusScopeNode();
    addTearDown(scope.dispose);
    FocusScope scene(bool allowed) => FocusScope(
      node: scope,
      canRequestFocus: allowed,
      child: const SizedBox.expand(),
    );
    await tester.pumpWidget(scene(true));
    scope.requestFocus();
    await tester.pump();
    expect(scope.hasPrimaryFocus, isTrue);

    final discarded = scene(false);
    expect(discarded.canRequestFocus, isFalse);
    expect(scope.canRequestFocus, isTrue);
    await tester.pump();
    expect(scope.hasPrimaryFocus, isTrue);

    await tester.pumpWidget(scene(false));
    await tester.pump();
    expect(scope.canRequestFocus, isFalse);
    expect(scope.hasPrimaryFocus, isFalse);
  });
}
