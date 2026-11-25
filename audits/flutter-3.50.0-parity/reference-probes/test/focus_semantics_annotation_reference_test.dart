import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

List<({String label, bool focusable, bool action, bool focused})> _nodes(
  WidgetTester tester,
) {
  final root = tester.binding.pipelineOwner.semanticsOwner!.rootSemanticsNode!;
  final result =
      <({String label, bool focusable, bool action, bool focused})>[];
  bool visit(SemanticsNode node) {
    final data = node.getSemanticsData();
    result.add((
      label: data.label,
      focusable: data.flagsCollection.isFocused != ui.Tristate.none,
      action: data.hasAction(ui.SemanticsAction.focus),
      focused: data.flagsCollection.isFocused == ui.Tristate.isTrue,
    ));
    node.visitChildren(visit);
    return true;
  }

  root.visitChildren(visit);
  return result;
}

Widget _app(Widget child) =>
    Directionality(textDirection: TextDirection.rtl, child: child);

void main() {
  testWidgets(
    'Focus annotations with two labels or independently active children',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final outer = FocusNode();
      final inner = FocusNode();
      addTearDown(outer.dispose);
      addTearDown(inner.dispose);

      await tester.pumpWidget(
        _app(
          Focus(
            focusNode: outer,
            child: const Column(children: [Text('one'), Text('two')]),
          ),
        ),
      );
      expect(_nodes(tester), [
        (label: 'one\ntwo', focusable: true, action: true, focused: false),
      ]);

      await tester.pumpWidget(
        _app(
          Focus(
            focusNode: outer,
            child: Focus(focusNode: inner, child: const Text('nested')),
          ),
        ),
      );
      expect(_nodes(tester), [
        (label: '', focusable: true, action: true, focused: false),
        (label: 'nested', focusable: true, action: true, focused: false),
      ]);

      await tester.pumpWidget(
        _app(
          Focus(
            focusNode: outer,
            child: Semantics(
              container: true,
              label: 'separate control',
              onTap: () {},
              child: const SizedBox(width: 20, height: 20),
            ),
          ),
        ),
      );
      expect(_nodes(tester), [
        (label: '', focusable: true, action: true, focused: false),
        (
          label: 'separate control',
          focusable: false,
          action: false,
          focused: false,
        ),
      ]);
      semantics.dispose();
    },
  );

  testWidgets(
    'Focus annotation separates focusability from the iOS focus action',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final node = FocusNode();
      addTearDown(node.dispose);
      await tester.pumpWidget(
        _app(Focus(focusNode: node, child: const Text('Test'))),
      );
      final hasAction = defaultTargetPlatform != TargetPlatform.iOS;
      expect(_nodes(tester), [
        (label: 'Test', focusable: true, action: hasAction, focused: false),
      ]);
      node.requestFocus();
      await tester.pumpAndSettle();
      expect(_nodes(tester), [
        (label: 'Test', focusable: true, action: hasAction, focused: true),
      ]);
      node.canRequestFocus = false;
      await tester.pumpAndSettle();
      expect(_nodes(tester), [
        (label: 'Test', focusable: false, action: false, focused: false),
      ]);
      semantics.dispose();
    },
    variant: const TargetPlatformVariant({
      TargetPlatform.linux,
      TargetPlatform.iOS,
    }),
  );
}
