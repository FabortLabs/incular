import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Select All and Copy follow screen order when widget order is reversed',
    (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.linux;
      addTearDown(() => debugDefaultTargetPlatformOverride = null);
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      String? clipboard;
      String? selection;
      messenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
        if (call.method == 'Clipboard.setData') {
          clipboard = (call.arguments as Map)['text'] as String?;
        }
        return null;
      });
      addTearDown(
        () => messenger.setMockMethodCallHandler(SystemChannels.platform, null),
      );

      for (final reverse in <bool>[false, true]) {
        final labels = <Widget>[
          const Positioned(left: 20, top: 16, child: Text('Left')),
          const Positioned(left: 140, top: 16, child: Text('Right')),
        ];
        await tester.pumpWidget(
          TestWidgetsApp(
            home: SelectableRegion(
              key: ValueKey<bool>(reverse),
              selectionControls: emptyTextSelectionControls,
              onSelectionChanged: (content) => selection = content?.plainText,
              child: SizedBox(
                width: 240,
                height: 75,
                child: Stack(
                  children: reverse ? labels.reversed.toList() : labels,
                ),
              ),
            ),
          ),
        );
        clipboard = null;
        selection = null;
        await tester.tap(find.text('Left'), kind: PointerDeviceKind.mouse);
        await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
        await tester.pump();
        expect(selection, 'LeftRight');
        expect(clipboard, 'LeftRight');
      }
    },
  );
}
