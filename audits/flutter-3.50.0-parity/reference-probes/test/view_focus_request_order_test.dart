import 'dart:ui';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a later widget focus request wins over directional view entry', (
    tester,
  ) async {
    final a = FocusNode(debugLabel: 'a');
    final b = FocusNode(debugLabel: 'b');
    addTearDown(a.dispose);
    addTearDown(b.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Column(children: [
          Focus(focusNode: a, child: const SizedBox(height: 50)),
          Focus(focusNode: b, child: const SizedBox(height: 50)),
        ]),
      ),
    );
    void event(ViewFocusState state, ViewFocusDirection direction) {
      ServicesBinding.instance.platformDispatcher.onViewFocusChange?.call(
        ViewFocusEvent(viewId: tester.view.viewId, state: state, direction: direction),
      );
    }

    event(ViewFocusState.unfocused, ViewFocusDirection.undefined);
    await tester.pump();
    event(ViewFocusState.focused, ViewFocusDirection.forward);
    b.requestFocus();
    await tester.pump();
    expect(b.hasPrimaryFocus, isTrue);
    expect(a.hasPrimaryFocus, isFalse);
  });
}
