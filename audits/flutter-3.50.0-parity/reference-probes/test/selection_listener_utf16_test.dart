import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart'
    show RenderParagraph, SelectedContentRange, SelectionStatus;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('selected ranges count UTF-16 units across labels', (
    tester,
  ) async {
    const first = ValueKey<String>('first');
    const second = ValueKey<String>('second');
    final focus = FocusNode();
    final notifier = SelectionListenerNotifier();
    addTearDown(focus.dispose);
    addTearDown(notifier.dispose);
    String? selected;
    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xff303030),
        pageRouteBuilder: <T>(RouteSettings settings, WidgetBuilder builder) =>
            PageRouteBuilder<T>(
              settings: settings,
              pageBuilder: (context, _, _) => builder(context),
            ),
        home: SelectableRegion(
          focusNode: focus,
          selectionControls: emptyTextSelectionControls,
          onSelectionChanged: (content) => selected = content?.plainText,
          child: SelectionListener(
            selectionNotifier: notifier,
            child: const Column(
              children: [
                Text('é😀x', key: first),
                Text('Zπ', key: second),
              ],
            ),
          ),
        ),
      ),
    );
    Offset point(Key key, int codeUnit) {
      final paragraph = tester.renderObject<RenderParagraph>(
        find.descendant(of: find.byKey(key), matching: find.byType(RichText)),
      );
      final caret = paragraph.getOffsetForCaret(
        TextPosition(offset: codeUnit),
        const Rect.fromLTWH(0, 0, 2, 20),
      );
      return paragraph.localToGlobal(
        Offset(caret.dx, paragraph.size.height / 2),
      );
    }

    final mouse = await tester.startGesture(
      point(first, 1),
      kind: PointerDeviceKind.mouse,
    );
    addTearDown(mouse.removePointer);
    await tester.pump();
    await mouse.moveTo(point(second, 1));
    await tester.pumpAndSettle();
    expect(notifier.selection.status, SelectionStatus.uncollapsed);
    expect(
      notifier.selection.range,
      const SelectedContentRange(startOffset: 1, endOffset: 5),
    );
    expect(selected, '😀xZ');
    await mouse.up();
  });
}
