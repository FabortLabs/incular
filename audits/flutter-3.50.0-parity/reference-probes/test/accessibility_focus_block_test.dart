import 'dart:ui' show Tristate;

import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('accessibility focus block preserves real keyboard focus', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final node = FocusNode();
    addTearDown(node.dispose);
    var keyDowns = 0;
    try {
      Widget scene(AccessibilityFocusBlockType mode) => Directionality(
        textDirection: TextDirection.ltr,
        child: Semantics(
          label: 'parent scope',
          container: true,
          explicitChildNodes: true,
          accessibilityFocusBlockType: mode,
          child: Semantics(
            label: 'keyboard child',
            container: true,
            focused: true,
            accessibilityFocusBlockType: AccessibilityFocusBlockType.none,
            child: Focus(
              focusNode: node,
              includeSemantics: false,
              onKeyEvent: (_, event) {
                if (event is KeyDownEvent &&
                    event.logicalKey == LogicalKeyboardKey.keyA) {
                  keyDowns++;
                  return KeyEventResult.handled;
                }
                return KeyEventResult.ignored;
              },
              child: const SizedBox(width: 40, height: 40),
            ),
          ),
        ),
      );
      SemanticsNode semantic(String label) =>
          tester.semantics.find(find.bySemanticsLabel(label));
      await tester.pumpWidget(scene(AccessibilityFocusBlockType.none));
      node.requestFocus();
      await tester.pumpAndSettle();
      expect(node.hasPrimaryFocus, isTrue);
      final childId = semantic('keyboard child').id;
      expect(
        semantic('keyboard child').getSemanticsData().flagsCollection.isFocused,
        Tristate.isTrue,
      );

      var modeCount = 0;
      for (final mode in [
        AccessibilityFocusBlockType.blockNode,
        AccessibilityFocusBlockType.blockSubtree,
        AccessibilityFocusBlockType.none,
      ]) {
        await tester.pumpWidget(scene(mode));
        final child = semantic('keyboard child');
        final flags = child.getSemanticsData().flagsCollection;
        final parentFlags = semantic('parent scope')
            .getSemanticsData()
            .flagsCollection;
        final inherited = mode == AccessibilityFocusBlockType.blockSubtree;
        expect(
          parentFlags.isAccessibilityFocusBlocked,
          mode != AccessibilityFocusBlockType.none,
        );
        expect(flags.isAccessibilityFocusBlocked, inherited);
        expect(flags.isFocused, inherited ? Tristate.none : Tristate.isTrue);
        expect(child.id, childId);
        expect(node.hasPrimaryFocus, isTrue);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
        expect(keyDowns, ++modeCount);
      }
    } finally {
      handle.dispose();
    }
  });
}
