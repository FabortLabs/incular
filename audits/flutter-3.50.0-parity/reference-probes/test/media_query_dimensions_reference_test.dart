import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('width, height, size and orientation notify independently', (
    tester,
  ) async {
    final runs = <(int, Object?)>[];
    final readers = <Object? Function(BuildContext)>[
      MediaQuery.sizeOf,
      MediaQuery.maybeSizeOf,
      MediaQuery.widthOf,
      MediaQuery.maybeWidthOf,
      MediaQuery.heightOf,
      MediaQuery.maybeHeightOf,
      MediaQuery.orientationOf,
      MediaQuery.maybeOrientationOf,
    ];
    final child = Directionality(
      textDirection: TextDirection.ltr,
      child: Column(
        children: [
          for (final (index, reader) in readers.indexed)
            Builder(
              builder: (context) {
                runs.add((index, reader(context)));
                return const SizedBox.shrink();
              },
            ),
        ],
      ),
    );
    Future<void> pump(double width, double height, Brightness brightness) =>
        tester.pumpWidget(
          MediaQuery(
            data: MediaQueryData(
              size: Size(width, height),
              platformBrightness: brightness,
            ),
            child: child,
          ),
        );
    List<(int, Object?)> read(List<int> indices) {
      final copy = [...runs];
      runs.clear();
      expect(copy.map((entry) => entry.$1).toList()..sort(), indices);
      return copy;
    }

    await pump(200, 100, Brightness.light);
    expect(
      read([0, 1, 2, 3, 4, 5, 6, 7]),
      contains((6, Orientation.landscape)),
    );
    await pump(260, 100, Brightness.light);
    read([0, 1, 2, 3]);
    await pump(260, 180, Brightness.light);
    read([0, 1, 4, 5]);
    await pump(260, 260, Brightness.light);
    expect(read([0, 1, 4, 5, 6, 7]), contains((6, Orientation.portrait)));
    await pump(260, 300, Brightness.light);
    read([0, 1, 4, 5]);
    await pump(260, 300, Brightness.dark);
    read([]);
  });
}
