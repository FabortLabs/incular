import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'packets during reassembly reach the rebuilt callbacks in order',
    (tester) async {
      var generation = 'before';
      final log = <String>[];
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: Builder(
            builder: (context) {
              final label = generation;
              return Listener(
                behavior: HitTestBehavior.opaque,
                onPointerDown: (event) => log.add('$label down'),
                onPointerUp: (event) => log.add('$label up'),
              );
            },
          ),
        ),
      );
      final sendPacket = tester.binding.platformDispatcher.onPointerDataPacket!;
      final x = 40 * tester.view.devicePixelRatio;
      void contact(int pointer) {
        final sequence = [ui.PointerChange.down, ui.PointerChange.up];
        sendPacket(
          ui.PointerDataPacket(
            data: [
              for (final (index, change) in sequence.indexed)
                ui.PointerData(
                  viewId: tester.view.viewId,
                  kind: ui.PointerDeviceKind.touch,
                  change: change,
                  timeStamp: Duration(milliseconds: pointer * 10 + index),
                  device: pointer,
                  pointerIdentifier: pointer,
                  physicalX: x,
                  physicalY: x,
                  buttons: change == ui.PointerChange.down ? kPrimaryButton : 0,
                ),
            ],
          ),
        );
      }

      contact(1);
      expect(log, ['before down', 'before up']);
      generation = 'after';
      final reload = tester.binding.reassembleApplication();
      expect(tester.binding.locked, isTrue);
      contact(2);
      expect(log, ['before down', 'before up']);
      await tester.pump();
      await reload;
      expect(log, ['before down', 'before up', 'after down', 'after up']);
    },
  );
}
