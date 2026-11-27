import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const first = ValueKey<String>('first');
  const last = ValueKey<String>('last');
  const style = TextStyle(fontSize: 10);
  for (final split in [false, true]) {
    for (final forward in [true, false]) {
      testWidgets(
        'captured mouse drag crosses text edges (split: $split, forward: $forward)',
        (tester) async {
          final focus = FocusNode();
          addTearDown(focus.dispose);
          String? selected;
          await tester.pumpWidget(
            WidgetsApp(
              color: const Color(0xff303030),
              pageRouteBuilder:
                  <T>(RouteSettings settings, WidgetBuilder builder) =>
                      PageRouteBuilder<T>(
                        settings: settings,
                        pageBuilder: (context, _, _) => builder(context),
                      ),
              home: SelectableRegion(
                focusNode: focus,
                selectionControls: emptyTextSelectionControls,
                onSelectionChanged: (content) => selected = content?.plainText,
                child: Center(
                  child: split
                      ? const Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text('ABCD', key: first, style: style),
                            SizedBox(height: 24),
                            Text('EFGH', key: last, style: style),
                          ],
                        )
                      : const Text('ABCD\nEFGH', key: first, style: style),
                ),
              ),
            ),
          );
          final leading = tester.getRect(find.byKey(first));
          final trailing = split ? tester.getRect(find.byKey(last)) : leading;
          final start = forward
              ? leading.topLeft + const Offset(1, 5)
              : trailing.bottomRight - const Offset(1, 5);
          final goal = split
              ? Offset(
                  start.dx,
                  forward ? trailing.bottom + 30 : leading.top - 30,
                )
              : Offset(
                  forward ? trailing.right + 30 : leading.left - 30,
                  start.dy,
                );
          final mouse = await tester.startGesture(
            start,
            kind: PointerDeviceKind.mouse,
          );
          if (split) {
            await mouse.moveTo(
              Offset(start.dx, (leading.bottom + trailing.top) / 2),
            );
            await tester.pump();
            expect(selected, forward ? 'ABCD' : 'EFGH');
          }
          await mouse.moveTo(goal);
          await tester.pump();
          await mouse.up();
          await tester.pump();
          expect(selected, split ? 'ABCDEFGH' : 'ABCD\nEFGH');
        },
      );
    }
  }
}
