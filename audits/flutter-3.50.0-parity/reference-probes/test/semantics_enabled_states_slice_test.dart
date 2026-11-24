import 'dart:ui' show Tristate;

import 'package:flutter/widgets.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('enabled can be unset, true, or false', (t) async {
    final semantics = t.ensureSemantics();
    try {
      for (final (flag, want) in [
        (null, Tristate.none),
        (true, Tristate.isTrue),
        (false, Tristate.isFalse),
      ]) {
        await t.pumpWidget(
          Semantics(
            container: true,
            label: 'Action',
            button: true,
            textDirection: TextDirection.ltr,
            enabled: flag,
            onTap: () {},
            child: const SizedBox(width: 120, height: 40),
          ),
        );
        final node = t.getSemantics(find.bySemanticsLabel('Action'));
        expect(node.getSemanticsData().flagsCollection.isEnabled, want);
      }
    } finally {
      semantics.dispose();
    }
  });
}
