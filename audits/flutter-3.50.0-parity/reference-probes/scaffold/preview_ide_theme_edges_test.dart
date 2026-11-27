import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:widget_preview_scaffold/src/theme/ide_theme.dart';
import 'package:widget_preview_scaffold/src/theme/theme.dart';

void main() {
  test('host colors keep CSS alpha and use exact luminance thresholds', () {
    final query = IdeThemeQueryParams({
      'backgroundColor': '%23AbC4',
      'foregroundColor': '#A0b1c280',
      'theme': 'light',
    });
    expect(query.backgroundColor, const Color.fromARGB(0x44, 0xaa, 0xbb, 0xcc));
    expect(query.foregroundColor, const Color.fromARGB(0x80, 0xa0, 0xb1, 0xc2));
    expect(query.darkMode, isFalse);
    expect(IdeThemeQueryParams({'theme': 'Light'}).darkMode, isTrue);
    expect(isValidDarkColor(const Color.fromARGB(127, 97, 97, 97)), isTrue);
    expect(isValidDarkColor(const Color.fromARGB(127, 98, 98, 98)), isFalse);
    expect(
      isValidLightColor(const Color.fromARGB(127, 241, 241, 241)),
      isFalse,
    );
    expect(isValidLightColor(const Color.fromARGB(127, 242, 242, 242)), isTrue);
  });
}
