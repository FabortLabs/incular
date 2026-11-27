import 'dart:ui' as ui;

import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

List<SemanticsData> annotations(WidgetTester tester) {
  final root = tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
  final results = <SemanticsData>[];
  root.visitChildren((child) {
    results.add(child.getSemanticsData());
    return true;
  });
  return results;
}

void main() {
  testWidgets('KeyboardListener can hide only its accessibility annotation', (
    WidgetTester tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final node = FocusNode();
    addTearDown(node.dispose);
    final events = <KeyEvent>[];
    final key = GlobalKey();
    Widget listener({bool includeSemantics = true}) => KeyboardListener(
      focusNode: node,
      includeSemantics: includeSemantics,
      onKeyEvent: events.add,
      child: SizedBox.expand(key: key),
    );

    await tester.pumpWidget(listener());
    expect(Focus.maybeOf(key.currentContext!), same(node));
    final unfocused = annotations(tester).single;
    expect(unfocused.flagsCollection.isFocused, ui.Tristate.isFalse);
    expect(unfocused.hasAction(ui.SemanticsAction.focus), isTrue);

    node.requestFocus();
    await tester.pumpAndSettle();
    final focused = annotations(tester).single;
    expect(focused.flagsCollection.isFocused, ui.Tristate.isTrue);
    expect(focused.hasAction(ui.SemanticsAction.focus), isTrue);

    await tester.pumpWidget(listener(includeSemantics: false));
    expect(node.hasPrimaryFocus, isTrue);
    expect(annotations(tester), isEmpty);
    await tester.sendKeyEvent(LogicalKeyboardKey.metaLeft);
    expect(events, [isA<KeyDownEvent>(), isA<KeyUpEvent>()]);
    semantics.dispose();
  });
}
