import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final maintainAnimation in [false, true]) {
    testWidgets(
      'Visibility only mutes its retained subtree when allowed: $maintainAnimation',
      (tester) async {
        final ticksEnabled = <bool>[];
        final reader = Builder(
          builder: (context) {
            final enabled = TickerMode.of(context);
            ticksEnabled.add(enabled);
            return Text(enabled ? 'enabled' : 'muted');
          },
        );
        Future<void> pump({
          required bool parentEnabled,
          required bool visible,
        }) => tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: TickerMode(
              enabled: parentEnabled,
              child: Visibility(
                visible: visible,
                maintainState: true,
                maintainAnimation: maintainAnimation,
                child: reader,
              ),
            ),
          ),
        );
        await pump(parentEnabled: true, visible: true);
        final enabled = find.text('enabled', skipOffstage: false);
        final muted = find.text('muted', skipOffstage: false);
        final original = tester.element(enabled);
        await pump(parentEnabled: true, visible: false);
        expect(
          tester.element(maintainAnimation ? enabled : muted),
          same(original),
        );
        await pump(parentEnabled: false, visible: true);
        expect(tester.element(muted), same(original));
        await pump(parentEnabled: true, visible: true);
        expect(tester.element(enabled), same(original));
        expect(ticksEnabled, [true, false, true]);
      },
    );
  }
}
