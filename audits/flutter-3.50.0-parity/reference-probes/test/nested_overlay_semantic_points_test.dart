import 'dart:ui' show SemanticsAction;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('visible nested overlay controls activate at their view points', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(800, 600);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    final handle = tester.ensureSemantics();

    final trace = <String>[];
    final outer = OverlayPortalController()..show();
    final inner = OverlayPortalController()..show();
    Widget button(String label) => Semantics(
      container: true,
      button: true,
      label: label,
      onTap: () => trace.add(label),
      child: const SizedBox(width: 8, height: 6),
    );

    final entry = OverlayEntry(
      builder: (_) => Align(
        alignment: Alignment.topLeft,
        child: OverlayPortal(
          controller: outer,
          child: button('a'),
          overlayChildBuilder: (_) => Positioned(
            left: 10,
            top: 11,
            child: OverlayPortal(
              controller: inner,
              child: button('b'),
              overlayChildBuilder: (_) =>
                  Positioned(left: 110, top: 211, child: button('c')),
            ),
          ),
        ),
      ),
    );
    addTearDown(() {
      entry.remove();
      entry.dispose();
    });
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: Overlay(initialEntries: [entry]),
      ),
    );

    final owner = tester.binding.pipelineOwner.semanticsOwner!;
    for (final point in const [
      Offset(1, 1),
      Offset(11, 12),
      Offset(111, 212),
    ]) {
      owner.performActionAt(point, SemanticsAction.tap);
    }
    handle.dispose();
    expect(trace, ['a', 'b', 'c']);
  });
}
