import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final autofocusFirst in [true, false]) {
    testWidgets(
      'sibling-scope autofocus and explicit request settle once (autofocus first: $autofocusFirst)',
      (WidgetTester tester) async {
        final a = FocusNode();
        final b = FocusNode();
        final left = FocusScopeNode();
        final right = FocusScopeNode();
        for (final node in [a, b, left, right]) {
          addTearDown(node.dispose);
        }
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Column(
              children: [
                FocusScope(
                  node: left,
                  child: Focus(focusNode: a, child: const SizedBox.shrink()),
                ),
                FocusScope(
                  node: right,
                  child: Focus(focusNode: b, child: const SizedBox.shrink()),
                ),
              ],
            ),
          ),
        );

        final manager = tester.binding.focusManager;
        final settled = <FocusNode?>[];
        final aEvents = <bool>[];
        void onPrimary() => settled.add(manager.primaryFocus);
        manager.addListener(onPrimary);
        addTearDown(() => manager.removeListener(onPrimary));
        a.addListener(() => aEvents.add(a.hasFocus));

        if (autofocusFirst) {
          right.autofocus(b);
          a.requestFocus();
        } else {
          a.requestFocus();
          right.autofocus(b);
        }
        expect(settled, isEmpty);
        await tester.pump();
        expect(settled, [b]);
        expect(aEvents, isEmpty);
        expect(b.hasPrimaryFocus, isTrue);
        expect(left.focusedChild, a);
        expect(right.focusedChild, b);
      },
    );
  }
  for (final queueAutofocus in [true, false]) {
    testWidgets(
      'superseded requests remember the inactive scope leaf (queued autofocus: $queueAutofocus)',
      (WidgetTester tester) async {
        final a1 = FocusNode();
        final a2 = FocusNode();
        final b = FocusNode();
        final left = FocusScopeNode();
        final right = FocusScopeNode();
        for (final node in [a1, a2, b, left, right]) {
          addTearDown(node.dispose);
        }
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Column(
              children: [
                FocusScope(
                  node: left,
                  child: Column(
                    children: [
                      Focus(focusNode: a1, child: const SizedBox.shrink()),
                      Focus(focusNode: a2, child: const SizedBox.shrink()),
                    ],
                  ),
                ),
                FocusScope(
                  node: right,
                  child: Focus(focusNode: b, child: const SizedBox.shrink()),
                ),
              ],
            ),
          ),
        );
        final manager = tester.binding.focusManager;
        final settled = <FocusNode?>[];
        void onPrimary() => settled.add(manager.primaryFocus);
        manager.addListener(onPrimary);
        addTearDown(() => manager.removeListener(onPrimary));

        a1.requestFocus();
        b.requestFocus();
        if (queueAutofocus) {
          left.autofocus(a2);
        }
        expect(settled, isEmpty);
        await tester.pump();
        expect(settled, [b]);
        expect(left.focusedChild, a1);
        expect(right.focusedChild, b);

        left.requestFocus();
        await tester.pump();
        expect(settled, [b, a1]);
        expect(a1.hasPrimaryFocus, isTrue);
      },
    );
  }
}
