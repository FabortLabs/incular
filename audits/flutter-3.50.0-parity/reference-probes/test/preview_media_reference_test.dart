import 'package:flutter/widget_previews.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

typedef _Snapshot = (Size, double, Brightness, double);

_Snapshot _observe(BuildContext context) => (
  MediaQuery.sizeOf(context),
  MediaQuery.textScalerOf(context).scale(1),
  MediaQuery.platformBrightnessOf(context),
  MediaQuery.paddingOf(context).top,
);

final class _MarkTheme extends PreviewThemeData {
  const _MarkTheme(this.observe);
  final void Function(_Snapshot) observe;

  @override
  Widget apply(BuildContext context, Widget child) {
    observe(_observe(context));
    return child;
  }
}

// This is a neutral probe of the public Flutter descriptors and values.
// The actual preview scaffold requires separate design-system packages.
Widget _compose(Preview preview, Widget child) {
  final theme = preview.theme?.call();
  final size = preview.size;
  final wrapped = preview.wrapper?.call(child) ?? child;
  return Builder(
    builder: (context) {
      return MediaQuery(
        data: MediaQuery.of(context).copyWith(
          size: size,
          textScaler: preview.textScaleFactor == null
              ? null
              : TextScaler.linear(preview.textScaleFactor!),
          platformBrightness: preview.brightness,
        ),
        child: SizedBox(
          width: size?.width == double.infinity ? null : size?.width,
          height: size?.height == double.infinity ? null : size?.height,
          child: Builder(
            builder: (context) => theme?.apply(context, wrapped) ?? wrapped,
          ),
        ),
      );
    },
  );
}

void main() {
  testWidgets('simulated media reaches theme and wrapper inside layout', (
    tester,
  ) async {
    final seen = <(String, _Snapshot)>[];
    const key = ValueKey('leaf');
    final preview = Preview(
      size: const Size.fromHeight(60),
      textScaleFactor: 1.75,
      brightness: Brightness.dark,
      theme: () => _MarkTheme((state) => seen.add(('theme', state))),
      wrapper: (child) => Builder(
        builder: (context) {
          seen.add(('wrapper', _observe(context)));
          return child;
        },
      ),
    );
    final child = _compose(preview, const SizedBox(key: key, width: 40));
    Future<void> pump(double top) => tester.pumpWidget(
      MediaQuery(
        data: MediaQueryData(
          size: const Size(500, 300),
          padding: EdgeInsets.only(top: top),
        ),
        child: Center(child: child),
      ),
    );

    await pump(12);
    expect(tester.getSize(find.byKey(key)), const Size(40, 60));
    final identity = tester.element(find.byKey(key));
    _Snapshot expected(double top) =>
        (preview.size!, 1.75, Brightness.dark, top);
    expect(seen, [('theme', expected(12)), ('wrapper', expected(12))]);
    await pump(20);
    expect(tester.element(find.byKey(key)), same(identity));
    expect(
      seen.skip(2),
      containsAll([('theme', expected(20)), ('wrapper', expected(20))]),
    );
  });

  testWidgets('unspecified preview media aspects stay inherited', (
    tester,
  ) async {
    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(
          size: Size(320, 240),
          devicePixelRatio: 2,
          platformBrightness: Brightness.dark,
          viewPadding: EdgeInsets.only(bottom: 8),
        ),
        child: _compose(
          const Preview(textScaleFactor: 2),
          Builder(
            builder: (context) {
              expect(MediaQuery.sizeOf(context), const Size(320, 240));
              expect(MediaQuery.devicePixelRatioOf(context), 2);
              expect(MediaQuery.platformBrightnessOf(context), Brightness.dark);
              expect(MediaQuery.viewPaddingOf(context).bottom, 8);
              expect(
                MediaQuery.textScalerOf(context),
                const TextScaler.linear(2),
              );
              return const SizedBox();
            },
          ),
        ),
      ),
    );
  });
}
