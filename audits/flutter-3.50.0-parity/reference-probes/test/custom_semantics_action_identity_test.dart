import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('canonical identity includes the override hint and standard action', () {
    const tap = CustomSemanticsAction.overridingAction(
      action: SemanticsAction.tap,
      hint: 'Select',
    );
    final equivalent = CustomSemanticsAction.overridingAction(
      action: SemanticsAction.tap,
      hint: ['Sel', 'ect'].join(),
    );
    const otherAction = CustomSemanticsAction.overridingAction(
      action: SemanticsAction.longPress,
      hint: 'Select',
    );
    const otherHint = CustomSemanticsAction.overridingAction(
      action: SemanticsAction.tap,
      hint: 'Choose',
    );
    const label = CustomSemanticsAction(label: 'Select');

    final id = CustomSemanticsAction.getIdentifier(tap);
    expect(CustomSemanticsAction.getIdentifier(equivalent), id);
    for (final other in [otherAction, otherHint, label]) {
      expect(CustomSemanticsAction.getIdentifier(other), isNot(id));
    }

    final recovered = CustomSemanticsAction.getAction(id)!;
    expect(recovered.label, isNull);
    expect(recovered.hint, 'Select');
    expect(recovered.action, SemanticsAction.tap);
    expect(label.label, 'Select');
    expect(label.hint, isNull);
    expect(label.action, isNull);
    expect(CustomSemanticsAction.getAction(-1), isNull);
  });
}
