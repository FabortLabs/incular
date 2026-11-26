import 'dart:ui' show Tristate;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Declared input focus does not create a keyboard focus target', (
    WidgetTester tester,
  ) async {
    const ValueKey<String> key = ValueKey<String>('annotation');
    Widget scene({bool? focused, bool? focusable}) => Directionality(
      textDirection: TextDirection.ltr,
      child: CustomScrollView(
        slivers: <Widget>[
          SliverSemantics(
            key: key,
            container: true,
            label: 'annotation',
            focused: focused,
            focusable: focusable,
            sliver: const SliverToBoxAdapter(child: SizedBox(height: 20)),
          ),
        ],
      ),
    );
    final FocusNode? keyboardFocus = FocusManager.instance.primaryFocus;
    int? retained;
    for (final (Widget widget, Tristate flag) in <(Widget, Tristate)>[
      (scene(), Tristate.none),
      (scene(focused: false), Tristate.isFalse),
      (scene(focused: true), Tristate.isTrue),
      (scene(focusable: true), Tristate.isFalse),
      (scene(focusable: false), Tristate.none),
      (scene(), Tristate.none),
    ]) {
      await tester.pumpWidget(widget);
      final SemanticsNode node = tester.getSemantics(find.byKey(key));
      retained ??= node.id;
      expect(node.id, retained);
      expect(node.getSemanticsData().flagsCollection.isFocused, flag);
      expect(node.getSemanticsData().hasAction(SemanticsAction.focus), isFalse);
      expect(FocusManager.instance.primaryFocus, keyboardFocus);
    }
  });
}
