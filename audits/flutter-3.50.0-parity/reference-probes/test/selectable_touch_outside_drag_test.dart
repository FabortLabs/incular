import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const source = 'alpha beta\nomega zeta';
  const key = ValueKey<String>('text');
  for (final forward in [true, false]) {
    testWidgets(
      'long-press extends whole words past the label (forward: $forward)',
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
              child: const Center(
                child: Text(source, key: key, style: TextStyle(fontSize: 10)),
              ),
            ),
          ),
        );
        final bounds = tester.getRect(find.byKey(key));
        final start = forward
            ? bounds.topLeft + const Offset(15, 5)
            : bounds.bottomRight - const Offset(15, 5);
        final goal = Offset(
          forward ? bounds.right + 30 : bounds.left - 30,
          start.dy,
        );
        final touch = await tester.startGesture(
          start,
          kind: PointerDeviceKind.touch,
        );
        await tester.pump(kLongPressTimeout);
        expect(selected, forward ? 'alpha' : 'zeta');
        await touch.moveTo(goal);
        await tester.pump();
        expect(selected, source);
        await touch.up();
        await tester.pump();
        expect(selected, source);
      },
    );
  }
}
