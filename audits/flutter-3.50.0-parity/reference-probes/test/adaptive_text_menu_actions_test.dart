// Authored for Incular parity; not an upstream test registration.

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Handles extends EmptyTextSelectionControls
    with TextSelectionHandleControls {}

void main() {
  testWidgets(
    'editable menu orders mobile actions and sends exact selected text',
    (WidgetTester tester) async {
      final controller = TextEditingController(text: '  word  rest');
      final focus = FocusNode();
      final key = GlobalKey<EditableTextState>();
      final calls = <(String, Object?)>[];
      final messenger = tester.binding.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(SystemChannels.platform, (
        MethodCall call,
      ) async {
        if (call.method == 'Clipboard.hasStrings') {
          return <String, bool>{'value': false};
        }
        if (call.method.endsWith('.invoke')) {
          calls.add((call.method, call.arguments));
        }
        return null;
      });
      var shown = <ContextMenuButtonItem>[];
      var obscure = false;
      var readOnly = false;
      ToolbarOptions? toolbarOptions;
      final controls = _Handles();
      final entry = OverlayEntry(
        builder: (BuildContext context) => Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 400,
            child: EditableText(
              key: key,
              controller: controller,
              focusNode: focus,
              autofocus: true,
              readOnly: readOnly,
              obscureText: obscure,
              selectionControls: controls,
              toolbarOptions: toolbarOptions,
              style: const TextStyle(fontFamily: 'Ahem', fontSize: 10),
              cursorColor: const Color(0xFF0000FF),
              backgroundCursorColor: const Color(0xFF808080),
              contextMenuBuilder:
                  (BuildContext context, EditableTextState state) {
                    shown = state.contextMenuButtonItems;
                    return const SizedBox.shrink(key: ValueKey('mounted menu'));
                  },
            ),
          ),
        ),
      );
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0xFFFFFFFF),
          builder: (context, child) => Overlay(initialEntries: [entry]),
        ),
      );
      await tester.pump();
      final android = defaultTargetPlatform == TargetPlatform.android;
      final ios = defaultTargetPlatform == TargetPlatform.iOS;
      const selection = TextSelection(baseOffset: 8, extentOffset: 0);
      controller.selection = selection;
      await tester.pump();
      expect(key.currentState!.showToolbar(), isTrue);
      await tester.pump();
      expect(shown.map((ContextMenuButtonItem item) => item.type), [
        ContextMenuButtonType.cut,
        ContextMenuButtonType.copy,
        if (android) ContextMenuButtonType.share,
        if (!ios) ContextMenuButtonType.selectAll,
        if (ios) ContextMenuButtonType.lookUp,
        if (ios) ContextMenuButtonType.searchWeb,
        if (ios) ContextMenuButtonType.share,
      ]);
      for (final item in shown) {
        if ({
          ContextMenuButtonType.lookUp,
          ContextMenuButtonType.searchWeb,
          ContextMenuButtonType.share,
        }.contains(item.type)) {
          item.onPressed!();
        }
      }
      await tester.pump();
      expect(calls, [
        if (ios) ('LookUp.invoke', '  word  '),
        if (ios) ('SearchWeb.invoke', '  word  '),
        if (ios || android) ('Share.invoke', '  word  '),
      ]);
      expect(controller.selection, selection);
      expect(focus.hasPrimaryFocus, isTrue);
      expect(find.byKey(const ValueKey('mounted menu')), findsOneWidget);

      final state = key.currentState!;
      bool hasAdaptiveActions() => state.contextMenuButtonItems.any(
        (item) => {
          ContextMenuButtonType.lookUp,
          ContextMenuButtonType.searchWeb,
          ContextMenuButtonType.share,
        }.contains(item.type),
      );
      controller.value = const TextEditingValue(
        text: '\uFEFF\u00A0word',
        selection: TextSelection(baseOffset: 0, extentOffset: 2),
      );
      await tester.pump();
      expect(hasAdaptiveActions(), isFalse);
      controller.selection = const TextSelection.collapsed(offset: 3);
      await tester.pump();
      expect(hasAdaptiveActions(), isFalse);
      obscure = true;
      entry.markNeedsBuild();
      await tester.pump();
      controller.selection = const TextSelection(
        baseOffset: 2,
        extentOffset: 6,
      );
      await tester.pump();
      expect(hasAdaptiveActions(), isFalse);
      obscure = false;
      readOnly = true;
      entry.markNeedsBuild();
      await tester.pump();
      controller.selection = const TextSelection(
        baseOffset: 2,
        extentOffset: 6,
      );
      await tester.pump();
      expect(hasAdaptiveActions(), android || ios);
      toolbarOptions = const ToolbarOptions(copy: true);
      entry.markNeedsBuild();
      await tester.pump();
      expect(hasAdaptiveActions(), isFalse);

      entry.remove();
      entry.dispose();
      await tester.pumpWidget(const SizedBox.shrink());
      messenger.setMockMethodCallHandler(SystemChannels.platform, null);
      controller.dispose();
      focus.dispose();
    },
    variant: const TargetPlatformVariant({
      TargetPlatform.android,
      TargetPlatform.iOS,
      TargetPlatform.linux,
    }),
  );
}
