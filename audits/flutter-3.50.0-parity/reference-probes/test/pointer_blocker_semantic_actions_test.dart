import 'dart:ui' show SemanticsAction;

import 'package:flutter/rendering.dart' show SemanticsNode;
import 'package:flutter/semantics.dart' show CustomSemanticsAction;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final kind in ['absorb', 'ignore', 'visibility']) {
    testWidgets('$kind gates user actions but not retained focus', (
      tester,
    ) async {
      final semantics = tester.ensureSemantics();
      try {
        final focus = FocusNode();
        addTearDown(focus.dispose);
        const key = ValueKey('action-node');
        final leaf = Semantics(
          key: key,
          container: true,
          label: 'Action node',
          focusable: true,
          onFocus: focus.requestFocus,
          onTap: () {},
          onLongPress: () {},
          onIncrease: () {},
          onDecrease: () {},
          onScrollLeft: () {},
          onDidGainAccessibilityFocus: () {},
          onDidLoseAccessibilityFocus: () {},
          customSemanticsActions: {
            const CustomSemanticsAction(label: 'custom action'): () {},
          },
          child: Focus(
            focusNode: focus,
            includeSemantics: false,
            child: const SizedBox(width: 60, height: 40),
          ),
        );
        Future<void> pump(bool blocked) {
          final wrapper = switch (kind) {
            'absorb' => AbsorbPointer(absorbing: blocked, child: leaf),
            'ignore' => IgnorePointer(ignoring: blocked, child: leaf),
            _ => Visibility(
              visible: !blocked,
              maintainState: true,
              maintainAnimation: true,
              maintainSize: true,
              maintainSemantics: true,
              maintainFocusability: true,
              child: leaf,
            ),
          };
          return tester.pumpWidget(
            Directionality(textDirection: TextDirection.ltr, child: wrapper),
          );
        }

        await pump(false);
        focus.requestFocus();
        await tester.pump();
        SemanticsNode? retained;
        for (final blocked in [false, true, false]) {
          await pump(blocked);
          final node = tester.getSemantics(
            find.byKey(key, skipOffstage: false),
          );
          retained ??= node;
          expect(node, same(retained));
          expect(focus.hasPrimaryFocus, isTrue);
          final data = node.getSemanticsData();
          expect(data.label, 'Action node');
          for (final action in [
            SemanticsAction.focus,
            SemanticsAction.tap,
            SemanticsAction.longPress,
            SemanticsAction.increase,
            SemanticsAction.decrease,
            SemanticsAction.scrollLeft,
            SemanticsAction.customAction,
          ]) {
            expect(data.hasAction(action), !blocked, reason: '$action');
          }
          expect(
            data.hasAction(SemanticsAction.didGainAccessibilityFocus),
            true,
          );
          expect(
            data.hasAction(SemanticsAction.didLoseAccessibilityFocus),
            true,
          );
        }
      } finally {
        semantics.dispose();
      }
    });
  }
}
