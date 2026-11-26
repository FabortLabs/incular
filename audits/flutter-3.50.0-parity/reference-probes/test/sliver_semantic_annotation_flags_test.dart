import 'dart:ui' show CheckedState, Tristate;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Independent annotation flags can coexist and be withdrawn', (
    WidgetTester tester,
  ) async {
    const ValueKey<String> key = ValueKey<String>('flags');
    int? retained;
    for (final bool active in <bool>[true, false, true]) {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            slivers: <Widget>[
              SliverSemantics(
                key: key,
                container: true,
                explicitChildNodes: true,
                label: 'flags',
                button: active,
                slider: active,
                keyboardKey: active,
                link: active,
                textField: active,
                inMutuallyExclusiveGroup: active,
                header: active,
                image: active,
                scopesRoute: active,
                namesRoute: active,
                expanded: active,
                checked: active ? false : null,
                mixed: active ? true : null,
                toggled: active ? null : true,
                sliver: const SliverToBoxAdapter(child: SizedBox(height: 20)),
              ),
            ],
          ),
        ),
      );
      final SemanticsNode node = tester.getSemantics(find.byKey(key));
      retained ??= node.id;
      expect(node.id, retained);
      final flags = node.getSemanticsData().flagsCollection;
      expect(flags.isButton, active);
      expect(flags.isSlider, active);
      expect(flags.isKeyboardKey, active);
      expect(flags.isLink, active);
      expect(flags.isTextField, active);
      expect(flags.isInMutuallyExclusiveGroup, active);
      expect(flags.isHeader, active);
      expect(flags.isImage, active);
      expect(flags.scopesRoute, active);
      expect(flags.namesRoute, active);
      expect(flags.isExpanded, active ? Tristate.isTrue : Tristate.isFalse);
      expect(flags.isChecked, active ? CheckedState.mixed : CheckedState.none);
      expect(flags.isToggled, active ? Tristate.none : Tristate.isTrue);
    }
  });
}
