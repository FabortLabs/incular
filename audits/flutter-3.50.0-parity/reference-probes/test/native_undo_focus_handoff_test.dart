import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('native undo follows focus after microtasks without a frame', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final first = TextEditingController.fromValue(
      const TextEditingValue(
        text: 'one',
        selection: TextSelection.collapsed(offset: 3),
      ),
    );
    final second = TextEditingController.fromValue(
      const TextEditingValue(
        text: 'alpha',
        selection: TextSelection.collapsed(offset: 5),
      ),
    );
    final firstFocus = FocusNode();
    final secondFocus = FocusNode();
    final log = <MethodCall>[];
    final messenger = tester.binding.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(SystemChannels.undoManager, (
      MethodCall call,
    ) async {
      log.add(call);
      return null;
    });

    Widget editor(TextEditingController text, FocusNode focus) => SizedBox(
      height: 32,
      child: EditableText(
        controller: text,
        focusNode: focus,
        style: const TextStyle(),
        cursorColor: const Color(0xff000000),
        backgroundCursorColor: const Color(0xff000000),
      ),
    );

    Future<void> send(String direction) async {
      await messenger.handlePlatformMessage(
        'flutter/undomanager',
        const JSONMethodCodec().encodeMethodCall(
          MethodCall('UndoManagerClient.handleUndo', <String>[direction]),
        ),
        (_) {},
      );
    }

    const throttle = Duration(milliseconds: 600);
    try {
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: DefaultTextEditingShortcuts(
            child: Column(
              children: [
                editor(first, firstFocus),
                editor(second, secondFocus),
              ],
            ),
          ),
        ),
      );
      firstFocus.requestFocus();
      await tester.pump();
      await tester.pump(throttle);
      first.value = const TextEditingValue(
        text: 'two',
        selection: TextSelection.collapsed(offset: 3),
      );
      await tester.pump(throttle);

      secondFocus.requestFocus();
      await tester.pump();
      await tester.pump(throttle);
      second.value = const TextEditingValue(
        text: 'beta',
        selection: TextSelection.collapsed(offset: 4),
      );
      await tester.pump(throttle);
      expect(secondFocus.hasPrimaryFocus, isTrue);
      log.clear();

      firstFocus.requestFocus();
      await tester.idle();
      expect(firstFocus.hasPrimaryFocus, isTrue);
      await send('undo');
      expect(first.text, 'one');
      expect(second.text, 'beta');
      final state = log.lastWhere(
        (m) => m.method == 'UndoManager.setUndoState',
      );
      expect(state.arguments, {'canUndo': false, 'canRedo': true});

      secondFocus.requestFocus();
      await tester.idle();
      expect(secondFocus.hasPrimaryFocus, isTrue);
      await send('redo');
      expect(first.text, 'one');
      expect(second.text, 'beta');
    } finally {
      await tester.pumpWidget(const SizedBox.shrink());
      messenger.setMockMethodCallHandler(SystemChannels.undoManager, null);
      first.dispose();
      second.dispose();
      firstFocus.dispose();
      secondFocus.dispose();
    }
  });
}
