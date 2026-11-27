import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('outer invalid state stops at an explicit semantic boundary', (
    tester,
  ) async {
    const key = ValueKey<String>('validated');
    Widget scene(bool boundary) => Directionality(
      textDirection: TextDirection.ltr,
      child: Semantics(
        validationResult: SemanticsValidationResult.invalid,
        child: Semantics(
          key: key,
          container: boundary,
          validationResult: SemanticsValidationResult.valid,
          child: const Text('status'),
        ),
      ),
    );

    await tester.pumpWidget(scene(false));
    expect(
      tester.getSemantics(find.byKey(key)).getSemanticsData().validationResult,
      SemanticsValidationResult.invalid,
    );
    await tester.pumpWidget(scene(true));
    expect(
      tester.getSemantics(find.byKey(key)).getSemanticsData().validationResult,
      SemanticsValidationResult.valid,
    );
  });
}
