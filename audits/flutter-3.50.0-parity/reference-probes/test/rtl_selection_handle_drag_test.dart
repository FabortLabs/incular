import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Handles extends TextSelectionControls {
  static const right = ValueKey('right artwork');
  static const left = ValueKey('left artwork');

  @override
  Size getHandleSize(double textLineHeight) => const Size.square(22);

  @override
  Offset getHandleAnchor(TextSelectionHandleType type, double textLineHeight) =>
      type == TextSelectionHandleType.left ? const Offset(22, 0) : Offset.zero;

  @override
  Widget buildHandle(BuildContext context, TextSelectionHandleType type, double height,
      [VoidCallback? onTap]) =>
      SizedBox.square(
        key: type == TextSelectionHandleType.right ? right : left,
        dimension: 22,
      );

  @override
  Widget buildToolbar(
    BuildContext context,
    Rect region,
    double height,
    Offset position,
    List<TextSelectionPoint> endpoints,
    TextSelectionDelegate delegate,
    ValueListenable<ClipboardStatus>? clipboardStatus,
    Offset? secondaryTapPosition,
  ) => const SizedBox.shrink();
}

void main() {
  testWidgets('Dragging the RTL right artwork moves the logical start', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final controller = TextEditingController(text: 'אבגדהוזחט');
    final node = FocusNode();
    addTearDown(controller.dispose);
    addTearDown(node.dispose);

    await tester.pumpWidget(
      WidgetsApp(
        color: const Color(0xff000000),
        pageRouteBuilder: <T>(settings, builder) => PageRouteBuilder<T>(
          settings: settings,
          pageBuilder: (context, _, _) => builder(context),
        ),
        home: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 400,
            height: 80,
            child: EditableText(
              controller: controller,
              focusNode: node,
              textDirection: TextDirection.ltr,
              style: const TextStyle(fontSize: 32),
              cursorColor: const Color(0xff000000),
              backgroundCursorColor: const Color(0xff000000),
              showSelectionHandles: true,
              selectionControls: _Handles(),
            ),
          ),
        ),
      ),
    );
    node.requestFocus();
    await tester.pump();
    controller.selection = const TextSelection(baseOffset: 3, extentOffset: 7);
    await tester.pumpAndSettle();

    final render = tester.allRenderObjects.whereType<RenderEditable>().single;
    expect(
      render.getEndpointsForSelection(controller.selection).first.direction,
      TextDirection.rtl,
    );
    final right = find.byKey(_Handles.right);
    expect(right, findsOneWidget);
    final start = tester.getCenter(right);
    final first = render.localToGlobal(
      render.getLocalRectForCaret(const TextPosition(offset: 0)).topLeft,
    );
    await tester.dragFrom(start, Offset(first.dx - start.dx, 0));
    await tester.pumpAndSettle();
    expect(controller.selection.baseOffset, 0);
    expect(controller.selection.extentOffset, 7);
  });
}
